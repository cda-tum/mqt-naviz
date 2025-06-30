//! Converter from the [na][super]-format to the [naviz][naviz_parser]-format.

use std::{borrow::Cow, collections::HashMap};

use fraction::{ConstZero, Fraction};
use naviz_parser::input::concrete::{
    InstructionGroup, Instructions, RelativeTimeline, SetupInstruction, TimedInstruction,
};

use super::format::{Number, Operation, OperationArgs, OperationList, Position};

/// Options for [convert]
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConvertOptions<'a> {
    /// The prefix to name atoms with.
    /// Atoms will be numbered starting from zero and named as `<atom_prefix><number>`
    pub atom_prefix: Cow<'a, str>,
    pub global_zones: GlobalZoneNames<'a>,
}

impl Default for ConvertOptions<'_> {
    fn default() -> Self {
        Self {
            atom_prefix: "atom".into(),
            global_zones: Default::default(),
        }
    }
}

/// Names of the zones to use for global operations (i.e., operations that are not targeted)
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GlobalZoneNames<'a> {
    pub cz: Cow<'a, str>,
    pub ry: Cow<'a, str>,
    pub rz: Cow<'a, str>,
}

impl Default for GlobalZoneNames<'_> {
    fn default() -> Self {
        Self {
            cz: "zone_cz".into(),
            ry: "zone_ry".into(),
            rz: "zone_rz".into(),
        }
    }
}

impl<'a> GlobalZoneNames<'a> {
    /// Gets the global zone name for an operation
    /// or returns an [OperationConversionError::InvalidName] if the name is unknown.
    fn get(&'a self, name: &str) -> Result<&'a str, OperationConversionError> {
        match name {
            "cz" => Ok(&self.cz),
            "ry" => Ok(&self.ry),
            "rz" => Ok(&self.rz),
            _ => Err(OperationConversionError::InvalidName),
        }
    }
}

/// A cache for the names of the atoms,
/// as the `na`-format only references atoms by position.
/// Maps position to name
type PositionCache = HashMap<Position, String>;

/// Tries to converts an [na][super] [OperationList] to [naviz][naviz_parser] [Instructions]
pub fn convert(
    input: &OperationList<&str>,
    options: ConvertOptions,
) -> Result<Instructions, OperationConversionError> {
    let mut position_cache = PositionCache::new();
    let mut atom_counter = 0;

    let mut timeline = RelativeTimeline::new();
    let mut setup = Vec::new();

    for operation in input.iter() {
        if let OperationArgs::Init(positions) = &operation.args {
            // Init-instruction: Create new atom
            for &position in positions.iter() {
                setup.push(SetupInstruction::Atom {
                    position: position.into(),
                    id: create_id(
                        &options.atom_prefix,
                        &mut atom_counter,
                        position,
                        &mut position_cache,
                    ),
                });
            }
        } else {
            // Convert the operation and add to timeline
            timeline.push((
                false,
                Fraction::ZERO,
                InstructionGroup {
                    variable: false,
                    instructions: convert_operation(
                        operation,
                        &mut position_cache,
                        &options.global_zones,
                    )?,
                },
            ));
        }
    }

    Ok(Instructions {
        setup,
        instructions: vec![(Fraction::ZERO, timeline)],
        ..Default::default()
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OperationConversionError {
    /// Tried to convert something to a [TimedInstruction] that is not timed (e.g., `init`)
    NotATimedInstruction,
    /// The name of the instruction is unknown or otherwise invalid
    InvalidName,
    /// A position was referenced, but no atom is known to be at that position
    UnknownPosition,
    /// An operation has two [PositionList][super::format::PositionList]s as arguments,
    /// but their lengths do not match
    MismatchedArgumentLengths,
    /// Operation was passed an argument, but none was expected
    SuperfluousArgument,
    /// Operation was not passed an argument, but one was expected
    MissingArgument,
}

/// Tries to convert an [Operation] to a [TimedInstruction].
/// Will use and update the passed [PositionCache]
/// and use the passed [GlobalZoneNames].
pub fn convert_operation(
    operation: &Operation<&str>,
    position_cache: &mut PositionCache,
    global_zone_options: &GlobalZoneNames,
) -> Result<Vec<TimedInstruction>, OperationConversionError> {
    match &operation.args {
        // Init is not a timed instruction
        OperationArgs::Init(_) => Err(OperationConversionError::NotATimedInstruction),
        OperationArgs::Shuttle { from, to } => {
            assert_same_lengths(from, to)?;
            from.iter()
                .zip(to.iter())
                // first extract the ids from the source positions
                .map(|(f, t)| Ok((extract_id(f, position_cache)?, t)))
                // then materialize to break the pipeline
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                // then insert new positions
                // This process is required as an atom may be moved to the old position of another atom
                // which would then overwrite the cache of the other atom.
                // Therefore, the whole operation needs to read the old version of the cache.
                .inspect(|(name, to)| insert_id(**to, name.clone(), position_cache))
                .map(|(name, to)| match operation.name {
                    "load" => Ok(TimedInstruction::Load {
                        position: Some((*to).into()),
                        id: name,
                    }),
                    "store" => Ok(TimedInstruction::Store {
                        position: Some((*to).into()),
                        id: name,
                    }),
                    "move" => Ok(TimedInstruction::Move {
                        position: (*to).into(),
                        id: name,
                    }),
                    _ => Err(OperationConversionError::InvalidName),
                })
                .collect()
        }
        OperationArgs::Local { argument, targets } => targets
            .iter()
            .map(|target| get_id(target, position_cache))
            .collect::<Result<_, _>>()
            .and_then(|targets| convert_operation_instruction(operation.name, targets, *argument))
            .map(|i| vec![i]),
        OperationArgs::Global(argument) => convert_operation_instruction(
            operation.name,
            vec![global_zone_options.get(operation.name)?.to_string()],
            *argument,
        )
        .map(|i| [i].into()),
    }
}

/// Converts a single [OperationArgs::Local] or [OperationArgs::Global] operation
/// to an instruction.
fn convert_operation_instruction(
    name: &str,
    targets: Vec<String>,
    argument: Option<Number>,
) -> Result<TimedInstruction, OperationConversionError> {
    Ok(match name {
        "cz" => {
            if argument.is_some() {
                return Err(OperationConversionError::SuperfluousArgument);
            }
            TimedInstruction::Cz { targets }
        }
        "ry" => {
            if let Some(argument) = argument {
                TimedInstruction::Ry {
                    value: argument,
                    targets,
                }
            } else {
                return Err(OperationConversionError::MissingArgument);
            }
        }
        "rz" => {
            if let Some(argument) = argument {
                TimedInstruction::Rz {
                    value: argument,
                    targets,
                }
            } else {
                return Err(OperationConversionError::MissingArgument);
            }
        }
        _ => return Err(OperationConversionError::InvalidName),
    })
}

impl From<Position> for (Fraction, Fraction) {
    fn from(value: Position) -> Self {
        (value.x, value.y)
    }
}

/// Creates a new id to initialize the atom at the passed `position`
/// and insert it into the `position_cache`.
/// Will use and increment the passed `counter`.
fn create_id(
    prefix: &str,
    counter: &mut u64,
    position: Position,
    position_cache: &mut PositionCache,
) -> String {
    let name = format!("{prefix}{counter}");
    *counter += 1;
    position_cache.insert(position, name.clone());
    name
}

/// Removes the atom at the `position` out of the `position_cache` and returns its `id`.
///
/// For a version that keeps the atom, see [get_id].
fn extract_id(
    position: &Position,
    position_cache: &mut PositionCache,
) -> Result<String, OperationConversionError> {
    position_cache
        .remove(position)
        .ok_or(OperationConversionError::UnknownPosition)
}

/// Inserts the atom with the passed `id` into the `position_cache` at the passed `position`
fn insert_id(position: Position, id: String, position_cache: &mut PositionCache) {
    position_cache.insert(position, id);
}

/// Gets the id of the atom at the `position` in the `position_cache`.
///
/// For a version that also removes the atom, see [extract_id].
fn get_id(
    position: &Position,
    position_cache: &PositionCache,
) -> Result<String, OperationConversionError> {
    position_cache
        .get(position)
        .ok_or(OperationConversionError::UnknownPosition)
        .cloned()
}

/// Asserts that two slices have the same lengths
fn assert_same_lengths<A, B>(a: &[A], b: &[B]) -> Result<(), OperationConversionError> {
    if a.len() != b.len() {
        Err(OperationConversionError::MismatchedArgumentLengths)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use naviz_parser::input::concrete::{
        InstructionGroup, Instructions, SetupInstruction, TimedInstruction,
    };

    use crate::mqt::na::{
        convert::{ConvertOptions, GlobalZoneNames},
        format::{parse, Operation, OperationArgs, Position},
    };

    use super::convert;

    /// Check if the imported version matches the manually converted one
    #[test]
    fn example() {
        let parsed =
            parse(include_str!("../../../rsc/test/example.na")).expect("Failed to parse example");

        // Load manually converted example
        let expected =
            naviz_parser::input::lexer::lex(include_str!("../../../rsc/test/example.naviz"))
                .expect("Failed to lex example expectation");
        let expected = naviz_parser::input::parser::parse(&expected)
            .expect("Failed to parse example expectation");
        let expected = Instructions::new(expected).expect("Failed to convert example expectation");

        // Convert the example programmatically
        let converted = convert(
            &parsed,
            ConvertOptions {
                atom_prefix: "atom".into(),
                global_zones: GlobalZoneNames {
                    cz: "global_cz".into(),
                    ry: "global_ry".into(),
                    rz: "global_rz".into(),
                },
            },
        )
        .expect("Failed to convert example");

        assert_eq!(
            converted, expected,
            "Conversion did not produce expected result"
        );
    }

    #[test]
    fn instruction_position_list() {
        let atoms: Arc<[_]> = [
            Position {
                x: 9.into(),
                y: 8.into(),
            },
            Position {
                x: 1.into(),
                y: 2.into(),
            },
            Position {
                x: 8.into(),
                y: 8.into(),
            },
            Position {
                x: 0.into(),
                y: 0.into(),
            },
        ]
        .into();

        let input = [
            Operation {
                name: "init",
                args: OperationArgs::Init(atoms.clone()),
            },
            Operation {
                name: "ry",
                args: OperationArgs::Local {
                    argument: Some(57.into()),
                    targets: atoms,
                },
            },
        ]
        .into();

        let converted = convert(
            &input,
            ConvertOptions {
                atom_prefix: "atom".into(),
                global_zones: GlobalZoneNames {
                    cz: "global_cz".into(),
                    ry: "global_ry".into(),
                    rz: "global_rz".into(),
                },
            },
        )
        .expect("Failed to convert");

        let expected = Instructions {
            setup: vec![
                SetupInstruction::Atom {
                    position: (9.into(), 8.into()),
                    id: "atom0".to_string(),
                },
                SetupInstruction::Atom {
                    position: (1.into(), 2.into()),
                    id: "atom1".to_string(),
                },
                SetupInstruction::Atom {
                    position: (8.into(), 8.into()),
                    id: "atom2".to_string(),
                },
                SetupInstruction::Atom {
                    position: (0.into(), 0.into()),
                    id: "atom3".to_string(),
                },
            ],
            instructions: vec![(
                0.into(),
                vec![(
                    false,
                    0.into(),
                    InstructionGroup {
                        variable: false,
                        instructions: vec![TimedInstruction::Ry {
                            value: 57.into(),
                            targets: vec![
                                "atom0".to_string(),
                                "atom1".to_string(),
                                "atom2".to_string(),
                                "atom3".to_string(),
                            ],
                        }],
                    },
                )],
            )],
            ..Default::default()
        };

        assert_eq!(
            converted, expected,
            "Instruction with position list incorrectly converted."
        );
    }
}

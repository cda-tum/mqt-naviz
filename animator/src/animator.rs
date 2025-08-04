use std::{borrow::Cow, collections::VecDeque, sync::Arc};

use fraction::{ConstZero, Fraction};
use naviz_parser::{
    config::{
        machine::MachineConfig,
        visual::{
            LeftRightPosition, OperationConfigConfigConfig, TopBottomPosition, VisualConfig,
            ZoneConfigConfig,
        },
    },
    input::concrete::{InstructionGroup, Instructions, SetupInstruction, TimedInstruction},
};
use naviz_state::{
    config::{
        AtomsConfig, Config, FontConfig, GridConfig, GridLegendConfig, HPosition, LegendConfig,
        LegendEntry, LegendSection, LineConfig, TimeConfig, TrapConfig, VPosition, ZoneConfig,
    },
    state::{AtomState, State},
};
use regex::Regex;

use crate::{
    color::Color,
    interpolator::{
        Constant, ConstantJerkFixedAverageVelocity, ConstantJerkFixedMaxVelocity,
        ConstantTransitionPoint, Diagonal, DurationCalculable, MaxVelocity, Triangle,
    },
    position::Position,
    timeline::{Time, Timeline},
    to_float::ToFloat,
};

/// The timelines for a single atom
pub struct AtomTimelines {
    position: Timeline<(), Position, f32, Diagonal<ConstantJerkFixedAverageVelocity>>,
    overlay_color: Timeline<(), Color, f32, Triangle>,
    size: Timeline<(), f32, f32, Triangle>,
    shuttling: Timeline<ConstantTransitionPoint, bool, f32, Constant>,
}

impl AtomTimelines {
    /// Creates new AtomTimelines from the passed default values
    pub fn new(position: Position, overlay_color: Color, size: f32, shuttling: bool) -> Self {
        Self {
            position: Timeline::new_with_interpolation(
                position,
                Diagonal(ConstantJerkFixedAverageVelocity()),
            ),
            overlay_color: Timeline::new(overlay_color),
            size: Timeline::new(size),
            shuttling: Timeline::new(shuttling),
        }
    }

    /// Gets the values of these timelines at the passed time
    pub fn get(&self, time: Time) -> (Position, Color, f32, bool) {
        (
            self.position.get(time),
            self.overlay_color.get(time),
            self.size.get(time),
            self.shuttling.get(time),
        )
    }
}

/// An atom-state in the animator
struct Atom {
    /// id of the atom
    id: String,
    /// display name (label) of the atom
    name: String,
    /// the timelines of the atom
    timelines: AtomTimelines,
}

/// The animator.
/// Contains the calculated [Atom]-states and static [Config].
///
/// Use [Animator::state] to get the animated state
/// and [Animator::config] to get the [Config].
pub struct Animator {
    atoms: Vec<Atom>,
    config: Arc<Config>,

    /// The total durations of the animations
    duration: Fraction,

    machine: MachineConfig,
    visual: VisualConfig,
}

impl Animator {
    /// Creates the [Animator]: calculates the timelines and the [Config]
    pub fn new(machine: MachineConfig, visual: VisualConfig, input: Instructions) -> Self {
        // Create the atoms
        let mut atoms: Vec<_> = input
            .setup
            .iter()
            .map(|a| match a {
                SetupInstruction::Atom { position, id } => Atom {
                    id: id.clone(),
                    name: get_name(&visual.atom.legend.name, id),
                    timelines: AtomTimelines::new(
                        (*position).into(),
                        Color::default(),
                        visual.atom.radius.f32(),
                        false,
                    ),
                },
            })
            .collect();

        // Convert the `Vec`s to `VecDeque`s to allow popping from front
        let mut absolute_timeline: VecDeque<(_, VecDeque<_>)> = input
            .instructions
            .into_iter()
            .map(|(t, i)| (t, i.into()))
            .collect();

        // Extract the positions / coordinates
        let setup_positions = input.setup.iter().map(|a| match a {
            SetupInstruction::Atom { position, .. } => position,
        });
        let setup_xs = setup_positions.clone().map(|p| p.0);
        let setup_ys = setup_positions.clone().map(|p| p.0);

        // (tl_x, tl_y, br_x, br_y)
        let mut content_extent = (
            setup_xs.clone().min().unwrap_or_default(),
            setup_ys.clone().min().unwrap_or_default(),
            setup_xs.clone().max().unwrap_or_default(),
            setup_ys.clone().max().unwrap_or_default(),
        );

        let mut duration_total = Fraction::ZERO;

        // Animate the atoms
        while let Some((time, mut relative_timeline)) = absolute_timeline.pop_front() {
            if let Some((_, offset, group)) = relative_timeline.pop_front() {
                let InstructionGroup {
                    variable,
                    instructions,
                } = group;

                // Duration of the group
                let mut duration = Fraction::ZERO;
                // Start time of the group
                let start_time = time + offset;
                let start_time_f32 = start_time.f32();

                // The invariable duration (if not set to `variable`)
                let invariable_duration = (!variable).then_some(()).and_then(|()| {
                    instructions
                        .iter()
                        .map(|i| get_duration(i, &atoms, &machine, start_time))
                        .max()
                });

                for instruction in instructions {
                    // Duration of the current instruction (or the group if not `variable`)
                    let current_duration = invariable_duration.unwrap_or_else(|| {
                        get_duration(&instruction, &atoms, &machine, start_time)
                    });
                    let current_duration_f32 = current_duration.f32();
                    // Update duration of group
                    duration = duration.max(current_duration);

                    // update extent
                    if let Some(position) = get_position(&instruction) {
                        content_extent.0 = content_extent.0.min(position.0);
                        content_extent.1 = content_extent.1.min(position.1);
                        content_extent.2 = content_extent.2.max(position.0);
                        content_extent.3 = content_extent.3.max(position.1);
                    }

                    targeted(&mut atoms, &instruction, start_time, &machine).for_each(|a| {
                        insert_animation(
                            &mut a.timelines,
                            &instruction,
                            start_time_f32,
                            current_duration_f32,
                            &visual,
                        )
                    });
                }

                let next_from_start = relative_timeline
                    .front()
                    .map(|(x, _, _)| *x)
                    .unwrap_or_default();
                // Update duration of whole animation by duration of group
                duration_total = duration_total.max(start_time + duration);
                let next_time = if next_from_start {
                    start_time
                } else {
                    start_time + duration
                };
                let idx = absolute_timeline.binary_search_by_key(&&next_time, |(t, _)| t);
                let idx = match idx {
                    Ok(idx) => idx,
                    Err(idx) => idx,
                };
                absolute_timeline.insert(idx, (next_time, relative_timeline));
            }
        }

        // Grow content extent to fit zones and traps
        for (x, y) in machine
            .zone
            .iter()
            .flat_map(|z| [z.1.from, z.1.to])
            .chain(machine.trap.iter().map(|t| t.1.position))
        {
            content_extent.0 = content_extent.0.min(x);
            content_extent.1 = content_extent.1.min(y);
            content_extent.2 = content_extent.2.max(x);
            content_extent.3 = content_extent.3.max(y);
        }

        // Add margin to extent
        content_extent.0 -= visual.coordinate.margin;
        content_extent.1 -= visual.coordinate.margin;
        content_extent.2 += visual.coordinate.margin;
        content_extent.3 += visual.coordinate.margin;

        // The legend entries
        let mut legend_entries = Vec::new();
        if visual.zone.legend.display {
            legend_entries.push(LegendSection {
                name: visual.zone.legend.title.clone(),
                entries: machine
                    .zone
                    .iter()
                    .filter_map(|(id, _)| {
                        get_first_match_with_regex(&visual.zone.config, id)
                            .filter(|(_, zone)| !zone.name.is_empty())
                            .map(|(regex, zone)| LegendEntry {
                                text: regex.replace(id, &zone.name).into_owned(),
                                color: Some(zone.color.rgba()),
                            })
                    })
                    .collect(),
            });
        }
        if visual.operation.legend.display {
            legend_entries.push(LegendSection {
                name: visual.operation.legend.title.clone(),
                entries: [
                    (
                        &visual.operation.config.rz.name,
                        visual.operation.config.rz.color,
                    ),
                    (
                        &visual.operation.config.ry.name,
                        visual.operation.config.ry.color,
                    ),
                    (
                        &visual.operation.config.cz.name,
                        visual.operation.config.cz.color,
                    ),
                ]
                .into_iter()
                .filter(|(name, _)| !name.is_empty())
                .map(|(name, color)| LegendEntry {
                    text: name.clone(),
                    color: Some(color.rgba()),
                })
                .collect(),
            });
        }
        if visual.machine.legend.display {
            legend_entries.push(LegendSection {
                name: visual.machine.legend.title.clone(),
                entries: [
                    (&visual.machine.trap.name, visual.atom.trapped.color),
                    (&visual.machine.shuttle.name, visual.atom.shuttling.color),
                ]
                .into_iter()
                .filter(|(name, _)| !name.is_empty())
                .map(|(name, color)| LegendEntry {
                    text: name.clone(),
                    color: Some(color.rgba()),
                })
                .collect(),
            });
        }

        // Create static config
        let config = Config {
            machine: naviz_state::config::MachineConfig {
                grid: GridConfig {
                    step: (
                        visual.coordinate.tick.x.f32(),
                        visual.coordinate.tick.y.f32(),
                    ),
                    line: LineConfig {
                        width: visual.coordinate.tick.line.thickness.f32(),
                        segment_length: visual.coordinate.tick.line.dash.length.f32(),
                        duty: Into::<Fraction>::into(visual.coordinate.tick.line.dash.duty).f32(),
                        color: visual.coordinate.tick.color.rgba(),
                    },
                    display_ticks: visual.coordinate.tick.display,
                    legend: GridLegendConfig {
                        step: (
                            visual.coordinate.number.x.distance.f32(),
                            visual.coordinate.number.y.distance.f32(),
                        ),
                        font: FontConfig {
                            size: visual.coordinate.number.font.size.f32(),
                            color: visual.coordinate.number.font.color.rgba(),
                            family: visual.coordinate.number.font.family.to_owned(),
                        },
                        labels: (
                            visual.coordinate.axis.x.clone(),
                            visual.coordinate.axis.y.clone(),
                        ),
                        position: (
                            match visual.coordinate.number.x.position {
                                TopBottomPosition::Bottom => VPosition::Bottom,
                                TopBottomPosition::Top => VPosition::Top,
                            },
                            match visual.coordinate.number.y.position {
                                LeftRightPosition::Left => HPosition::Left,
                                LeftRightPosition::Right => HPosition::Right,
                            },
                        ),
                        display_labels: visual.coordinate.axis.display,
                        display_numbers: visual.coordinate.number.display,
                    },
                },
                traps: TrapConfig {
                    positions: machine
                        .trap
                        .values()
                        .map(|t| (t.position.0.f32(), t.position.1.f32()))
                        .collect(),
                    radius: visual.machine.trap.radius.f32(),
                    line_width: visual.machine.trap.line_width.f32(),
                    color: visual.machine.trap.color.rgba(),
                },
                zones: machine
                    .zone
                    .iter()
                    .map(|(id, zone)| {
                        let start: (f32, f32) = (zone.from.0.f32(), zone.from.1.f32());
                        let end: (f32, f32) = (zone.to.0.f32(), zone.to.1.f32());
                        let size = (end.0 - start.0, end.1 - start.1);
                        let default_line = ZoneConfigConfig {
                            color: naviz_parser::common::color::Color {
                                r: 0,
                                g: 0,
                                b: 0,
                                a: 0,
                            },
                            line: naviz_parser::config::visual::LineConfig {
                                dash: naviz_parser::config::visual::DashConfig {
                                    length: Default::default(),
                                    duty: naviz_parser::common::percentage::Percentage(
                                        Default::default(),
                                    ),
                                },
                                thickness: Default::default(),
                            },
                            name: "".to_owned(),
                        };
                        let line =
                            get_first_match(&visual.zone.config, id).unwrap_or(&default_line);
                        ZoneConfig {
                            start,
                            size,
                            line: LineConfig {
                                width: line.line.thickness.f32(),
                                segment_length: line.line.dash.length.f32(),
                                duty: line.line.dash.duty.0.f32(),
                                color: line.color.rgba(),
                            },
                        }
                    })
                    .collect(),
            },
            atoms: AtomsConfig {
                label: FontConfig {
                    size: visual.atom.legend.font.size.f32(),
                    color: visual.atom.legend.font.color.rgba(),
                    family: visual.atom.legend.font.family.to_owned(),
                },
                shuttle: LineConfig {
                    width: visual.machine.shuttle.line.thickness.f32(),
                    segment_length: visual.machine.shuttle.line.dash.length.f32(),
                    duty: Into::<Fraction>::into(visual.machine.shuttle.line.dash.duty).f32(),
                    color: visual.machine.shuttle.color.rgba(),
                },
            },
            content_extent: (
                (content_extent.0.f32(), content_extent.1.f32()),
                (content_extent.2.f32(), content_extent.3.f32()),
            ),
            legend: LegendConfig {
                font: FontConfig {
                    size: visual.sidebar.font.size.f32(),
                    color: visual.sidebar.font.color.rgba(),
                    family: visual.sidebar.font.family.to_owned(),
                },
                heading_skip: visual.sidebar.padding.heading.f32(),
                entry_skip: visual.sidebar.padding.entry.f32(),
                color_circle_radius: visual.sidebar.color_radius.f32(),
                color_padding: visual.sidebar.padding.color.f32(),
                entries: legend_entries,
            },
            time: TimeConfig {
                font: FontConfig {
                    size: visual.time.font.size.f32(),
                    color: visual.time.font.color.rgba(),
                    family: visual.time.font.family.clone(),
                },
                display: visual.time.display,
            },
        };

        Self {
            atoms,
            config: Arc::new(config),
            duration: duration_total,
            machine,
            visual,
        }
    }

    /// The calculated [Config]
    pub fn config(&self) -> Arc<Config> {
        self.config.clone()
    }

    /// The total duration of the animations in this [Animator]
    pub fn duration(&self) -> Fraction {
        self.duration
    }

    /// Gets the [State] at the passed [Time]
    pub fn state(&self, time: Time) -> State {
        State {
            atoms: self
                .atoms
                .iter()
                .map(
                    |Atom {
                         id: _,
                         name,
                         timelines,
                     }| (timelines.get(time), name),
                )
                .map(
                    |((position, overlay_color, size, shuttling), name)| AtomState {
                        position: position.into(),
                        size,
                        color: overlay_color
                            .over(&if shuttling {
                                self.visual.atom.shuttling.color.into()
                            } else {
                                self.visual.atom.trapped.color.into()
                            })
                            .0,
                        shuttle: shuttling,
                        label: name.clone(),
                    },
                )
                .collect(),
            time: self.format_time(time),
        }
    }

    /// The background color
    pub fn background(&self) -> [u8; 4] {
        self.visual.viewport.color.rgba()
    }

    /// Format the given [Time] into a time-string according to the [TimeConfig] in the current [VisualConfig].
    fn format_time(&self, time: Time) -> String {
        if !self.visual.time.display {
            // Don't display the time
            return String::new();
        }

        format!(
            "{}{:.*} {}",
            self.visual.time.prefix,
            self.visual.time.precision.f64().abs().floor() as usize,
            time,
            self.machine.time.unit,
        )
    }
}

/// Extracts the position of a [TimedInstruction],
/// if the instruction has a position,
/// otherwise returns [None].
fn get_position(instruction: &TimedInstruction) -> Option<(Fraction, Fraction)> {
    match instruction {
        TimedInstruction::Load { position, .. } | TimedInstruction::Store { position, .. } => {
            *position
        }
        TimedInstruction::Move { position, .. } => Some(*position),
        _ => None,
    }
}

/// Checks whether an `atom` is in the passed `zone` at the specified `time`.
fn is_in_zone(
    atom: &Atom,
    zone: &naviz_parser::config::machine::ZoneConfig,
    time: Fraction,
) -> bool {
    let position = atom.timelines.position.get(time.f32().into());
    position.x >= zone.from.0.f32()
        && position.y >= zone.from.1.f32()
        && position.x <= zone.to.0.f32()
        && position.y <= zone.to.1.f32()
}

/// Checks whether two positions `a` and `b` are at most `max_distance` apart.
fn is_close(a: &Position, b: &Position, max_distance: Fraction) -> bool {
    let distance_sq = (a.x - b.x).powi(2) + (a.y - b.y).powi(2);
    distance_sq <= max_distance.f32().powi(2)
}

/// Filters the passed `atoms`-slice to only contain the atoms that are targeted
/// by the  passed `instruction` at the specified `start_time` (time the instruction starts)
/// and returns an iterator over all qualifying atoms.
fn targeted<'a>(
    atoms: &'a mut [Atom],
    instruction: &'a TimedInstruction,
    start_time: Fraction,
    machine: &'a MachineConfig,
) -> impl Iterator<Item = &'a mut Atom> {
    enum Match<'a> {
        /// Match a single atom by ID
        Atom(&'a str),
        /// Match multiple atoms by id or zones by config
        AtomsOrZones {
            atoms: Vec<&'a str>,
            zones: Vec<&'a naviz_parser::config::machine::ZoneConfig>,
        },
        /// Match by index
        Index(Vec<usize>),
    }
    let m = match instruction {
        // Instructions that only target individual atoms
        TimedInstruction::Load { id, .. }
        | TimedInstruction::Store { id, .. }
        | TimedInstruction::Move { id, .. } => Match::Atom(id),
        // Instructions that target arbitrary targets
        TimedInstruction::Rz { targets, .. } | TimedInstruction::Ry { targets, .. } => {
            Match::AtomsOrZones {
                zones: targets
                    .iter()
                    .filter_map(|id| machine.zone.get(id))
                    .collect(),
                atoms: targets.iter().map(AsRef::as_ref).collect(),
            }
        }
        // Instructions that target arbitrary targets and require interaction distance
        TimedInstruction::Cz { targets, .. } => {
            let zones: Vec<_> = targets
                .iter()
                .filter_map(|id| machine.zone.get(id))
                .collect();

            // Get the position for each atom (identified by index) that is targeted at the start_time
            let in_zone: Vec<_> = atoms
                .iter()
                .enumerate()
                .filter(|(_, a)| {
                    targets.contains(&a.id)
                        || zones.iter().any(|zone| is_in_zone(a, zone, start_time))
                })
                .map(|(idx, a)| (idx, a.timelines.position.get(start_time.f32().into())))
                .collect();

            // Generate the targeted atom indices using nested loop.
            // Assuming that at any time only clusters of two atoms exist,
            // at most all atoms will be added to this vector
            // as no atom will be added multiple times in the below loop.
            // Therefore, we preallocate this size
            let mut targeted = Vec::with_capacity(in_zone.len());
            for a in 0..in_zone.len() {
                for b in (a + 1)..in_zone.len() {
                    let (a, a_pos) = &in_zone[a];
                    let (b, b_pos) = &in_zone[b];
                    // Two atoms are close -> add both
                    if is_close(a_pos, b_pos, machine.distance.interaction) {
                        targeted.push(*a);
                        targeted.push(*b);
                    }
                }
            }
            Match::Index(targeted)
        }
    };
    atoms
        .iter_mut()
        .enumerate()
        .filter(move |(idx, a)| match &m {
            Match::Atom(id) => &a.id == id,
            Match::AtomsOrZones { atoms, zones } => {
                atoms.contains(&&*a.id) || zones.iter().any(|zone| is_in_zone(a, zone, start_time))
            }
            Match::Index(indices) => indices.contains(idx),
        })
        .map(|(_, a)| a)
}

/// Gets the duration of the passed `instruction` when starting at the passed `time`,
fn get_duration(
    instruction: &TimedInstruction,
    atoms: &[Atom],
    machine: &MachineConfig,
    time: Fraction,
) -> Fraction {
    match instruction {
        TimedInstruction::Load { .. } => machine.time.load,
        TimedInstruction::Store { .. } => machine.time.store,
        TimedInstruction::Move { position, id } => (|| {
            let start = atoms
                .iter()
                .find(|a| &a.id == id)?
                .timelines
                .position
                .get(time.f32().into());
            let end = (position.0.f32(), position.1.f32());

            Some(
                Diagonal(ConstantJerkFixedMaxVelocity::new_fixed(MaxVelocity(
                    machine.movement.max_speed.f32(),
                )))
                .duration((), start, Position { x: end.0, y: end.1 }),
            )
        })()
        .map(Fraction::from)
        .unwrap_or_default(),
        TimedInstruction::Rz { .. } => machine.time.rz,
        TimedInstruction::Ry { .. } => machine.time.ry,
        TimedInstruction::Cz { .. } => machine.time.cz,
    }
}

/// Inserts an animation for the passed `instruction` into the passed `timelines`
fn insert_animation(
    timelines: &mut AtomTimelines,
    instruction: &TimedInstruction,
    start_time: f32,
    duration: f32,
    visual: &VisualConfig,
) {
    fn add_operation(
        timelines: &mut AtomTimelines,
        time: f32,
        duration: f32,
        config: &OperationConfigConfigConfig,
        visual: &VisualConfig,
    ) {
        timelines
            .overlay_color
            .add((time, duration, config.color.into()));
        timelines
            .size
            .add((time, duration, config.radius.get(visual.atom.radius).f32()));
    }

    fn add_move(
        timelines: &mut AtomTimelines,
        time: f32,
        duration: f32,
        target: (Fraction, Fraction),
    ) {
        let target: Position = target.into();
        timelines.position.add((time, duration, (), target));
    }

    fn add_load_store(
        timelines: &mut AtomTimelines,
        time: f32,
        duration: f32,
        load: bool,
        position: Option<(Fraction, Fraction)>,
    ) {
        if load {
            timelines
                .shuttling
                .add((time, duration, ConstantTransitionPoint::Start, true));
        } else {
            timelines
                .shuttling
                .add((time, duration, ConstantTransitionPoint::End, false));
        };
        if let Some(position) = position {
            add_move(timelines, time, duration, position);
        }
    }

    match instruction {
        TimedInstruction::Load { position, .. } => {
            add_load_store(timelines, start_time, duration, true, *position);
        }
        TimedInstruction::Store { position, .. } => {
            add_load_store(timelines, start_time, duration, false, *position);
        }
        TimedInstruction::Move { position, .. } => {
            add_move(timelines, start_time, duration, *position);
        }
        TimedInstruction::Rz { .. } => {
            add_operation(
                timelines,
                start_time,
                duration,
                &visual.operation.config.rz,
                visual,
            );
        }
        TimedInstruction::Ry { .. } => {
            add_operation(
                timelines,
                start_time,
                duration,
                &visual.operation.config.ry,
                visual,
            );
        }
        TimedInstruction::Cz { .. } => {
            add_operation(
                timelines,
                start_time,
                duration,
                &visual.operation.config.cz,
                visual,
            );
        }
    }
}

/// Gets a name based of an id (from a regex-string-map)
fn get_name(names: &[(Regex, String)], id: &str) -> String {
    names
        .iter()
        .find_map(|(regex, replace)| match regex.replace(id, replace) {
            Cow::Borrowed(_) => None, // borrowed => original input => did not match
            Cow::Owned(n) => Some(n),
        })
        .unwrap_or_default()
}

/// Gets the first entry of the passed `input`-map where the id matches the regex.
fn get_first_match_with_regex<'t, T>(input: &'t [(Regex, T)], id: &str) -> Option<&'t (Regex, T)> {
    input.iter().find(|(r, _)| r.is_match(id))
}

/// Gets the first item of the passed `input`-map where the id matches the regex.
fn get_first_match<'t, T>(input: &'t [(Regex, T)], id: &str) -> Option<&'t T> {
    get_first_match_with_regex(input, id).map(|(_, t)| t)
}

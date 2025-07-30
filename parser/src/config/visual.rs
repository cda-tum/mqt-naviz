//! The [VisualConfig] and sub-types.
//! See documentation of file-format.

use crate::common::{color::Color, percentage::Percentage};

use super::{
    error::{Error, ErrorKind},
    generic::{get_item, get_item_map, get_item_named_struct, get_item_struct, Config, ConfigItem},
    parser::Value,
};
use fraction::Fraction;
use regex::Regex;

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct VisualConfig {
    pub name: String,
    pub atom: AtomConfig,
    pub zone: ZoneConfig,
    pub operation: OperationConfig,
    pub machine: MachineConfig,
    pub coordinate: CoordinateConfig,
    pub sidebar: SidebarConfig,
    pub time: TimeConfig,
    pub viewport: ViewportConfig,
}

impl TryFrom<Config> for VisualConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            name: get_item(&mut value, "name")?,
            atom: get_item_struct(&mut value, "atom")?,
            zone: get_item_struct(&mut value, "zone")?,
            operation: get_item_struct(&mut value, "operation")?,
            machine: get_item_struct(&mut value, "machine")?,
            coordinate: get_item_struct(&mut value, "coordinate")?,
            sidebar: get_item_struct(&mut value, "sidebar")?,
            time: get_item_struct(&mut value, "time")?,
            viewport: get_item_struct(&mut value, "viewport")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct AtomConfig {
    pub trapped: TrappedConfig,
    pub shuttling: ShuttlingConfig,
    pub legend: AtomLegendConfig,
    pub radius: Fraction,
}

impl TryFrom<Config> for AtomConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            trapped: get_item_struct(&mut value, "trapped")?,
            shuttling: get_item_struct(&mut value, "shuttling")?,
            legend: get_item_struct(&mut value, "legend")?,
            radius: get_item(&mut value, "radius")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct TrappedConfig {
    pub color: Color,
}

impl TryFrom<Config> for TrappedConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            color: get_item(&mut value, "color")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct ShuttlingConfig {
    pub color: Color,
}

impl TryFrom<Config> for ShuttlingConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            color: get_item(&mut value, "color")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AtomLegendConfig {
    pub name: Vec<(Regex, String)>,
    pub font: FontConfig,
}

#[cfg(test)]
impl PartialEq for AtomLegendConfig {
    fn eq(&self, other: &Self) -> bool {
        self.font == other.font
            && self.name.len() == other.name.len()
            && self
                .name
                .iter()
                .zip(other.name.iter())
                .all(|((sk, sv), (ok, ov))| sk.as_str() == ok.as_str() && sv == ov)
    }
}

impl TryFrom<Config> for AtomLegendConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            name: get_item_map(&mut value, "name")?,
            font: get_item_struct(&mut value, "font")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct FontConfig {
    pub family: String,
    pub size: Fraction,
    pub color: Color,
}

impl TryFrom<Config> for FontConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            family: get_item(&mut value, "family")?,
            size: get_item(&mut value, "size")?,
            color: get_item(&mut value, "color")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ZoneConfig {
    pub config: Vec<(Regex, ZoneConfigConfig)>,
    pub legend: LegendConfig,
}

#[cfg(test)]
impl PartialEq for ZoneConfig {
    fn eq(&self, other: &Self) -> bool {
        self.legend == other.legend
            && self.config.len() == other.config.len()
            && self
                .config
                .iter()
                .zip(other.config.iter())
                .all(|((sk, sv), (ok, ov))| sk.as_str() == ok.as_str() && sv == ov)
    }
}

impl TryFrom<Config> for ZoneConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            config: get_item_named_struct(&mut value, "config", |v| match v {
                Value::Regex(r) => Some(r),
                _ => None,
            })?,
            legend: get_item_struct(&mut value, "legend")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct ZoneConfigConfig {
    pub color: Color,
    pub line: LineConfig,
    pub name: String,
}

impl TryFrom<Config> for ZoneConfigConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            color: get_item(&mut value, "color")?,
            line: get_item_struct(&mut value, "line")?,
            name: get_item(&mut value, "name")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct LineConfig {
    pub thickness: Fraction,
    pub dash: DashConfig,
}

impl TryFrom<Config> for LineConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            thickness: get_item(&mut value, "thickness")?,
            dash: get_item_struct(&mut value, "dash")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct DashConfig {
    pub length: Fraction,
    pub duty: Percentage,
}

impl TryFrom<Config> for DashConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            length: get_item(&mut value, "length")?,
            duty: get_item(&mut value, "duty")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct LegendConfig {
    pub display: bool,
    pub title: String,
}

impl TryFrom<Config> for LegendConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            display: get_item(&mut value, "display")?,
            title: get_item(&mut value, "title")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct OperationConfig {
    pub config: OperationConfigConfig,
    pub legend: LegendConfig,
}

impl TryFrom<Config> for OperationConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            config: get_item_struct(&mut value, "config")?,
            legend: get_item_struct(&mut value, "legend")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct OperationConfigConfig {
    pub ry: OperationConfigConfigConfig,
    pub rz: OperationConfigConfigConfig,
    pub cz: OperationConfigConfigConfig,
}

impl TryFrom<Config> for OperationConfigConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            ry: get_item_struct(&mut value, "ry")?,
            rz: get_item_struct(&mut value, "rz")?,
            cz: get_item_struct(&mut value, "cz")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct OperationConfigConfigConfig {
    pub color: Color,
    pub name: String,
    pub radius: NumberOrPercentage,
}

impl TryFrom<Config> for OperationConfigConfigConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            color: get_item(&mut value, "color")?,
            name: get_item(&mut value, "name")?,
            radius: get_item(&mut value, "radius")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone, Copy)]
pub enum NumberOrPercentage {
    Number(Fraction),
    Percentage(Percentage),
}

impl NumberOrPercentage {
    /// Returns the contained number if `self` is a [Number][NumberOrPercentage::Number]
    /// or the scaled `reference` if `self` is a [Percentage][NumberOrPercentage::Percentage].
    pub fn get(self, reference: Fraction) -> Fraction {
        match self {
            Self::Number(n) => n,
            Self::Percentage(p) => Fraction::from(p) * reference,
        }
    }
}

impl TryFrom<ConfigItem> for NumberOrPercentage {
    type Error = Error;
    fn try_from(value: ConfigItem) -> Result<Self, Self::Error> {
        match value {
            ConfigItem::Value(Value::Number(n)) => Ok(Self::Number(n)),
            ConfigItem::Value(Value::Percentage(n)) => Ok(Self::Percentage(n)),
            _ => Err(ErrorKind::WrongType("number | percentage").into()),
        }
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct MachineConfig {
    pub trap: TrapConfig,
    pub shuttle: ShuttleConfig,
    pub legend: LegendConfig,
}

impl TryFrom<Config> for MachineConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            trap: get_item_struct(&mut value, "trap")?,
            shuttle: get_item_struct(&mut value, "shuttle")?,
            legend: get_item_struct(&mut value, "legend")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct TrapConfig {
    pub color: Color,
    pub radius: Fraction,
    pub line_width: Fraction,
    pub name: String,
}

impl TryFrom<Config> for TrapConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            color: get_item(&mut value, "color")?,
            radius: get_item(&mut value, "radius")?,
            line_width: get_item(&mut value, "line_width")?,
            name: get_item(&mut value, "name")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct ShuttleConfig {
    pub color: Color,
    pub line: LineConfig,
    pub name: String,
}

impl TryFrom<Config> for ShuttleConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            color: get_item(&mut value, "color")?,
            line: get_item_struct(&mut value, "line")?,
            name: get_item(&mut value, "name")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct CoordinateConfig {
    pub tick: TickConfig,
    pub number: NumberConfig,
    pub axis: AxisConfig,
    pub margin: Fraction,
}

impl TryFrom<Config> for CoordinateConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            tick: get_item_struct(&mut value, "tick")?,
            number: get_item_struct(&mut value, "number")?,
            axis: get_item_struct(&mut value, "axis")?,
            margin: get_item(&mut value, "margin")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct TickConfig {
    pub x: Fraction,
    pub y: Fraction,
    pub color: Color,
    pub line: LineConfig,
    pub display: bool,
}

impl TryFrom<Config> for TickConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            x: get_item(&mut value, "x")?,
            y: get_item(&mut value, "y")?,
            color: get_item(&mut value, "color")?,
            line: get_item_struct(&mut value, "line")?,
            display: get_item(&mut value, "display")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct NumberConfig {
    pub x: NumberConfigConfig<TopBottomPosition>,
    pub y: NumberConfigConfig<LeftRightPosition>,
    pub display: bool,
    pub font: FontConfig,
}

impl TryFrom<Config> for NumberConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            x: get_item_struct(&mut value, "x")?,
            y: get_item_struct(&mut value, "y")?,
            display: get_item(&mut value, "display")?,
            font: get_item_struct(&mut value, "font")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct NumberConfigConfig<P> {
    pub distance: Fraction,
    pub position: P,
}

impl<T> TryFrom<Config> for NumberConfigConfig<T>
where
    ConfigItem: TryInto<T, Error = Error>,
{
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            distance: get_item::<Fraction>(&mut value, "distance")?,
            position: get_item(&mut value, "position")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub enum TopBottomPosition {
    Top,
    Bottom,
}

impl TryFrom<ConfigItem> for TopBottomPosition {
    type Error = Error;
    fn try_from(value: ConfigItem) -> Result<Self, Self::Error> {
        match value {
            ConfigItem::Value(Value::Identifier(s)) => match s.as_str() {
                "top" => Ok(Self::Top),
                "bottom" => Ok(Self::Bottom),
                _ => Err(ErrorKind::WrongType("'top' | 'bottom'").into()),
            },
            _ => Err(ErrorKind::WrongType("'top' | 'bottom'").into()),
        }
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub enum LeftRightPosition {
    Left,
    Right,
}

impl TryFrom<ConfigItem> for LeftRightPosition {
    type Error = Error;
    fn try_from(value: ConfigItem) -> Result<Self, Self::Error> {
        match value {
            ConfigItem::Value(Value::Identifier(s)) => match s.as_str() {
                "left" => Ok(Self::Left),
                "right" => Ok(Self::Right),
                _ => Err(ErrorKind::WrongType("'top' | 'bottom'").into()),
            },
            _ => Err(ErrorKind::WrongType("'top' | 'bottom'").into()),
        }
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct AxisConfig {
    pub x: String,
    pub y: String,
    pub display: bool,
    pub font: FontConfig,
}

impl TryFrom<Config> for AxisConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            x: get_item(&mut value, "x")?,
            y: get_item(&mut value, "y")?,
            display: get_item(&mut value, "display")?,
            font: get_item_struct(&mut value, "font")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct SidebarConfig {
    pub font: FontConfig,
    pub margin: Fraction,
    pub padding: SidebarPaddingConfig,
    pub color_radius: Fraction,
}

impl TryFrom<Config> for SidebarConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            font: get_item_struct(&mut value, "font")?,
            margin: get_item(&mut value, "margin")?,
            padding: get_item_struct(&mut value, "padding")?,
            color_radius: get_item(&mut value, "color_radius")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct SidebarPaddingConfig {
    pub color: Fraction,
    pub heading: Fraction,
    pub entry: Fraction,
}

impl TryFrom<Config> for SidebarPaddingConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            color: get_item(&mut value, "color")?,
            heading: get_item(&mut value, "heading")?,
            entry: get_item(&mut value, "entry")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct TimeConfig {
    pub display: bool,
    pub prefix: String,
    pub precision: Fraction,
    pub font: FontConfig,
}

impl TryFrom<Config> for TimeConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            display: get_item(&mut value, "display")?,
            prefix: get_item(&mut value, "prefix")?,
            precision: get_item(&mut value, "precision")?,
            font: get_item_struct(&mut value, "font")?,
        })
    }
}

#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Clone)]
pub struct ViewportConfig {
    pub margin: Fraction,
    pub color: Color,
}

impl TryFrom<Config> for ViewportConfig {
    type Error = Error;
    fn try_from(mut value: Config) -> Result<Self, Self::Error> {
        Ok(Self {
            margin: get_item(&mut value, "margin")?,
            color: get_item(&mut value, "color")?,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::{lexer, parser};

    #[test]
    fn example() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/rsc/test/example.nastyle"
        ));

        let expected = VisualConfig {
            name: "Example".to_string(),
            atom: AtomConfig {
                trapped: TrappedConfig {
                    color: Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 255,
                    },
                },
                shuttling: ShuttlingConfig {
                    color: Color {
                        r: 255,
                        g: 255,
                        b: 255,
                        a: 255,
                    },
                },
                legend: AtomLegendConfig {
                    name: vec![(Regex::new("^.*$").unwrap(), "$0".to_string())],
                    font: FontConfig {
                        family: "Nice Font".to_string(),
                        size: Fraction::new(14u64, 1u64),
                        color: Color {
                            r: 255,
                            g: 0,
                            b: 255,
                            a: 255,
                        },
                    },
                },
                radius: Fraction::new(32u64, 1u64),
            },
            zone: ZoneConfig {
                config: vec![
                    (
                        Regex::new("^zone.*$").unwrap(),
                        ZoneConfigConfig {
                            color: Color {
                                r: 0,
                                g: 0,
                                b: 255,
                                a: 255,
                            },
                            line: LineConfig {
                                thickness: Fraction::new(2u64, 1u64),
                                dash: DashConfig {
                                    length: Fraction::new(10u64, 1u64),
                                    duty: Percentage(Fraction::new(50u64, 1u64)),
                                },
                            },
                            name: "Cool zone".to_string(),
                        },
                    ),
                    (
                        Regex::new("^.*$").unwrap(),
                        ZoneConfigConfig {
                            color: Color {
                                r: 0,
                                g: 0,
                                b: 52,
                                a: 255,
                            },
                            line: LineConfig {
                                thickness: Fraction::new(1u64, 1u64),
                                dash: DashConfig {
                                    length: Fraction::new(5u64, 1u64),
                                    duty: Percentage(Fraction::new(20u64, 1u64)),
                                },
                            },
                            name: "Normal zone".to_string(),
                        },
                    ),
                ],
                legend: LegendConfig {
                    display: true,
                    title: "Zones".to_string(),
                },
            },
            operation: OperationConfig {
                config: OperationConfigConfig {
                    ry: OperationConfigConfigConfig {
                        color: Color {
                            r: 242,
                            g: 149,
                            b: 237,
                            a: 255,
                        },
                        name: "ry".to_string(),
                        radius: NumberOrPercentage::Number(Fraction::new(32u64, 1u64)),
                    },
                    rz: OperationConfigConfigConfig {
                        color: Color {
                            r: 18,
                            g: 52,
                            b: 86,
                            a: 255,
                        },
                        name: "rz".to_string(),
                        radius: NumberOrPercentage::Percentage(Percentage(Fraction::new(
                            48u64, 1u64,
                        ))),
                    },
                    cz: OperationConfigConfigConfig {
                        color: Color {
                            r: 192,
                            g: 255,
                            b: 238,
                            a: 255,
                        },
                        name: "cz".to_string(),
                        radius: NumberOrPercentage::Number(Fraction::new(13u64, 1u64)),
                    },
                },
                legend: LegendConfig {
                    display: true,
                    title: "Operations".to_string(),
                },
            },
            machine: MachineConfig {
                trap: TrapConfig {
                    color: Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 170,
                    },
                    radius: Fraction::new(18u64, 1u64),
                    line_width: Fraction::new(1u64, 1u64),
                    name: "Trap".to_string(),
                },
                shuttle: ShuttleConfig {
                    color: Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 204,
                    },
                    line: LineConfig {
                        thickness: Fraction::new(1u64, 1u64),
                        dash: DashConfig {
                            length: Fraction::new(10u64, 1u64),
                            duty: Percentage(Fraction::new(50u64, 1u64)),
                        },
                    },
                    name: "Shuttle".to_string(),
                },
                legend: LegendConfig {
                    display: true,
                    title: "".to_string(),
                },
            },
            coordinate: CoordinateConfig {
                tick: TickConfig {
                    x: Fraction::new(10u64, 1u64),
                    y: Fraction::new(20u64, 1u64),
                    color: Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 143,
                    },
                    line: LineConfig {
                        thickness: Fraction::new(1u64, 1u64),
                        dash: DashConfig {
                            length: Fraction::new(10u64, 1u64),
                            duty: Percentage(Fraction::new(50u64, 1u64)),
                        },
                    },
                    display: true,
                },
                number: NumberConfig {
                    x: NumberConfigConfig {
                        distance: Fraction::new(30u64, 1u64),
                        position: TopBottomPosition::Bottom,
                    },
                    y: NumberConfigConfig {
                        distance: Fraction::new(20u64, 1u64),
                        position: LeftRightPosition::Left,
                    },
                    display: true,
                    font: FontConfig {
                        family: "Font".to_string(),
                        size: Fraction::new(8u64, 1u64),
                        color: Color {
                            r: 0,
                            g: 0,
                            b: 0,
                            a: 255,
                        },
                    },
                },
                axis: AxisConfig {
                    x: "x".to_string(),
                    y: "y".to_string(),
                    display: true,
                    font: FontConfig {
                        family: "New Font".to_string(),
                        size: Fraction::new(18u64, 1u64),
                        color: Color {
                            r: 0,
                            g: 0,
                            b: 0,
                            a: 255,
                        },
                    },
                },
                margin: Fraction::new(12u64, 1u64),
            },
            sidebar: SidebarConfig {
                font: FontConfig {
                    family: "Yet another font".to_string(),
                    size: Fraction::new(10u64, 1u64),
                    color: Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 255,
                    },
                },
                margin: Fraction::new(8u64, 1u64),
                padding: SidebarPaddingConfig {
                    color: Fraction::new(24u64, 1u64),
                    heading: Fraction::new(48u64, 1u64),
                    entry: Fraction::new(32u64, 1u64),
                },
                color_radius: Fraction::new(8u64, 1u64),
            },
            time: TimeConfig {
                display: true,
                prefix: "Time: ".to_string(),
                precision: Fraction::new(1u64, 1u64),
                font: FontConfig {
                    family: "Last Font".to_string(),
                    size: Fraction::new(12u64, 1u64),
                    color: Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 255,
                    },
                },
            },
            viewport: ViewportConfig {
                margin: Fraction::new(4u64, 1u64),
                color: Color {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            },
        };

        let lexed = lexer::lex(input).expect("Failed to lex");
        let parsed = parser::parse(lexed.as_slice()).expect("Failed to parse");
        let generic: Config = parsed.into();
        let config: VisualConfig = generic.try_into().expect("Failed to load config");

        assert_eq!(config, expected);
    }
}

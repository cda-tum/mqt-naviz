use crate::{Color, Extent, Position, Size};

/// Static config (i.e., does not usually change)
#[derive(Clone, Debug)]
pub struct Config {
    /// The config for the machine
    pub machine: MachineConfig,
    /// The config for the atoms
    pub atoms: AtomsConfig,
    /// The config for the legend
    pub legend: LegendConfig,
    /// The config for the time
    pub time: TimeConfig,
    /// The extent of the content (in content-coordinates), denoted by top-left and bottom-right
    pub content_extent: Extent,
}

#[derive(Clone, Debug)]
pub struct MachineConfig {
    /// The config for the coordinate grid
    pub grid: GridConfig,
    /// The config for the traps
    pub traps: TrapConfig,
    /// The configs for the zones
    pub zones: Vec<ZoneConfig>,
}

#[derive(Clone, Debug)]
pub struct GridConfig {
    /// The distance between the lines in x- and y-direction
    pub step: Size,
    /// The config for the grid lines
    pub line: LineConfig,
    /// The config for the legend at the sides
    pub legend: GridLegendConfig,
    /// Whether to display the coordinate ticks
    pub display_ticks: bool,
}

#[derive(Clone, Debug)]
pub struct GridLegendConfig {
    /// The distance between the labels in x- and y-direction
    pub step: Size,
    /// The config for the font of the labels
    pub font: FontConfig,
    /// The labels for the x- and y-axis
    pub labels: (String, String),
    /// The position of the labels
    pub position: (VPosition, HPosition),
    /// Whether to display the labels for the x- and y-axis
    pub display_labels: bool,
    /// Whether to display the numbers on the axes
    pub display_numbers: bool,
}

#[derive(Clone, Debug)]
pub struct TrapConfig {
    /// The positions of the traps
    pub positions: Vec<Position>,
    /// The radius of a trap
    pub radius: f32,
    /// The line width of the trap-circles
    pub line_width: f32,
    /// The color of the traps
    pub color: Color,
}

#[derive(Clone, Copy, Debug)]
pub struct ZoneConfig {
    /// The top-left point of the zone
    pub start: Position,
    /// The size of the zone
    pub size: Size,
    /// The config of the line for this zone
    pub line: LineConfig,
}

#[derive(Clone, Debug)]
pub struct AtomsConfig {
    /// The config for the shuttles
    pub shuttle: LineConfig,
    /// The config for the labels of the atoms
    pub label: FontConfig,
}

#[derive(Clone, Debug)]
pub struct LegendConfig {
    /// The config for the font of the legend
    pub font: FontConfig,
    /// How much to skip before each heading.
    /// Includes the font size
    /// (i.e., a skip of `0` results in the next item overlapping the heading).
    pub heading_skip: f32,
    /// How much to skip before each entry.
    /// Includes the font size
    /// (i.e., a skip of `0` results in the next item overlapping the entry).
    pub entry_skip: f32,
    /// The radius of the circles showing the color
    pub color_circle_radius: f32,
    /// The padding between a circle and the text
    pub color_padding: f32,
    /// The legend entries
    pub entries: Vec<LegendSection>,
}

#[derive(Clone, Debug)]
pub struct LegendSection {
    /// The name of the section (i.e., the section-heading)
    pub name: String,
    /// The entries of this section
    pub entries: Vec<LegendEntry>,
}

#[derive(Clone, Debug)]
pub struct LegendEntry {
    /// The label of this entry
    pub text: String,
    /// The color for the circle next to the text.
    /// [None] will render no circle.
    pub color: Option<Color>,
}

#[derive(Clone, Debug)]
pub struct TimeConfig {
    /// The config for the font of the time-display
    pub font: FontConfig,
    /// Whether to display the current time
    pub display: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct LineConfig {
    /// The width of this line
    pub width: f32,
    /// The segment length of this line
    pub segment_length: f32,
    /// The fraction filled in each segment
    pub duty: f32,
    /// The color of this line
    pub color: Color,
}

#[derive(Clone, Debug)]
pub struct FontConfig {
    /// The size of the text
    pub size: f32,
    /// The color of the text
    pub color: Color,
    /// The font family / name for the text
    pub family: String,
}

/// A vertical position
#[derive(Clone, Copy, Debug)]
pub enum VPosition {
    Top,
    Bottom,
}

/// A vertical position ([Top][VPosition::Top] or [Bottom][VPosition::Bottom])
impl VPosition {
    /// Gets a value based on this [VPosition]
    #[inline]
    pub fn get<T>(&self, top: T, bottom: T) -> T {
        match self {
            Self::Top => top,
            Self::Bottom => bottom,
        }
    }

    /// Gets the inverse of this position
    #[inline]
    pub fn inverse(&self) -> Self {
        match self {
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
        }
    }
}

/// A horizontal position ([Left][HPosition::Left] or [Right][HPosition::Right])
#[derive(Clone, Copy, Debug)]
pub enum HPosition {
    Left,
    Right,
}

impl HPosition {
    /// Gets a value based on this [HPosition]
    #[inline]
    pub fn get<T>(&self, left: T, right: T) -> T {
        match self {
            Self::Left => left,
            Self::Right => right,
        }
    }

    /// Gets the inverse of this position
    #[inline]
    pub fn inverse(&self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

impl Config {
    /// Whether the time-area should be displayed
    pub fn display_time(&self) -> bool {
        self.time.display
    }

    /// Whether the sidebar should be displayed
    pub fn display_sidebar(&self) -> bool {
        !self.legend.entries.is_empty()
    }

    /// An example [Config]
    pub fn example() -> Self {
        Self {
            machine: MachineConfig {
                grid: GridConfig {
                    step: (20., 20.),
                    line: LineConfig {
                        width: 1.,
                        segment_length: 0.,
                        duty: 1.,
                        color: [127, 127, 127, 255],
                    },
                    display_ticks: true,
                    legend: GridLegendConfig {
                        step: (40., 40.),
                        font: FontConfig {
                            size: 12.,
                            color: [16, 16, 16, 255],
                            family: "Fira Mono".to_owned(),
                        },
                        labels: ("x".to_owned(), "y".to_owned()),
                        position: (VPosition::Bottom, HPosition::Left),
                        display_labels: true,
                        display_numbers: true,
                    },
                },
                traps: TrapConfig {
                    positions: (0..=7)
                        .map(|x| x as f32 * 14.)
                        .flat_map(|x| {
                            (0..=1)
                                .map(|y| y as f32 * 17.)
                                .chain((0..=2).map(|y| y as f32 * 17. + 38.))
                                .chain((0..=1).map(|y| y as f32 * 17. + 85.))
                                .map(move |y| (x, y))
                        })
                        .collect(),
                    radius: 3.,
                    line_width: 0.5,
                    color: [100, 100, 130, 255],
                },
                zones: vec![
                    ZoneConfig {
                        start: (-10., -10.),
                        size: (120., 36.),
                        line: LineConfig {
                            width: 1.,
                            segment_length: 0.,
                            duty: 1.,
                            color: [0, 122, 255, 255],
                        },
                    },
                    ZoneConfig {
                        start: (-10., 30.),
                        size: (120., 46.),
                        line: LineConfig {
                            width: 1.,
                            segment_length: 0.,
                            duty: 1.,
                            color: [255, 122, 0, 255],
                        },
                    },
                    ZoneConfig {
                        start: (-10., 80.),
                        size: (120., 36.),
                        line: LineConfig {
                            width: 1.,
                            segment_length: 0.,
                            duty: 1.,
                            color: [0, 122, 255, 255],
                        },
                    },
                ],
            },
            atoms: AtomsConfig {
                shuttle: LineConfig {
                    width: 1.,
                    segment_length: 10.,
                    duty: 0.5,
                    color: [180, 180, 180, 255],
                },
                label: FontConfig {
                    size: 5.,
                    color: [0, 0, 0, 255],
                    family: "Fira Mono".to_owned(),
                },
            },
            legend: LegendConfig {
                font: FontConfig {
                    size: 32.,
                    color: [0, 0, 0, 255],
                    family: "Fira Mono".to_owned(),
                },
                heading_skip: 80.,
                entry_skip: 64.,
                color_circle_radius: 16.,
                color_padding: 8.,
                entries: vec![
                    LegendSection {
                        name: "Zones".to_owned(),
                        entries: vec![
                            LegendEntry {
                                text: "Top".to_owned(),
                                color: Some([0, 122, 255, 255]),
                            },
                            LegendEntry {
                                text: "Middle".to_owned(),
                                color: Some([255, 122, 0, 255]),
                            },
                            LegendEntry {
                                text: "Bottom".to_owned(),
                                color: Some([0, 122, 255, 255]),
                            },
                        ],
                    },
                    LegendSection {
                        name: "Atoms".to_owned(),
                        entries: vec![LegendEntry {
                            text: "Atom".to_owned(),
                            color: Some([255, 128, 32, 255]),
                        }],
                    },
                    LegendSection {
                        name: "Foo".to_owned(),
                        entries: vec![LegendEntry {
                            text: "Bar".to_owned(),
                            color: None,
                        }],
                    },
                ],
            },
            time: TimeConfig {
                font: FontConfig {
                    size: 48.,
                    color: [0, 0, 0, 255],
                    family: "Fira Mono".to_owned(),
                },
                display: true,
            },
            content_extent: ((0., 0.), (100., 120.)),
        }
    }
}

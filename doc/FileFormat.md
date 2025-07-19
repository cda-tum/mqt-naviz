# File Format

## Types

| Type                   | Description                                                  | Examples                          |
| ---------------------- | ------------------------------------------------------------ | --------------------------------- |
| `number`               | Decimal                                                      | `42`, `0.2`, `-1.8`               |
| `id`                   | Alphanumeric or `_` identifier; Must not only contain digits | `zone0`, `Foo`, `atom_99`         |
| `percentage`           | Relative number; specified as percentage                     | `5%`, `-4%`, `3.8%`               |
| `color`                | Hex-color in `#RRGGBBAA` format; alpha is optional           | `#1eb69dcc`, `#ac52f6`            |
| `string`               | A string; Must be enclosed in double-quotes                  | `"Hello World!"`, `"Some string"` |
| `tuple(<a>, <b>, ...)` | A tuple; element-types are specified in parentheses          | `(5, 2)`                          |
| `set(<type>)`          | A set; element-type is specified in parentheses              | `{ atom0, atom1 }`                |
| `regex`                | A regex; Must be enclosed in `^`, `$`                        | `^atom.*$`, `^Foo:Bar$`           |
| `boolean`              | A boolean (`true`/`false`) value                             | `true`, `false`                   |

### Type aliases

| Alias      | Type                    | Description                                                                     |
| ---------- | ----------------------- | ------------------------------------------------------------------------------- |
| `position` | `tuple(number, number)` | A position                                                                      |
| `target`   | `id \| set(target)`     | A target (i.e., a zone, an atom, or a set containing multiple targets)          |
| `time`     | `number`                | A time; can also be one of the [relative times](#automatic-time--relative-time) |

## Machine Configuration

Configuration parameters for the machine are set in a `.namachine`-file.
The file-name will be the machine-id.
It contains multiple configuration-blocks:

### Display-name

A human-readable name for the machine can be set in the `name`-field.

```
name: <string> // Name of the machine
```

### Movement Speeds

The maximum movement speed of the machine is specified in the `movement`-block.
The machine will interpolate using constant jerk while respecting this `max_speed`.

```
movement {
	max_speed: <number>> // Max speed
}
```

### Times

The `time`-block allows setting the time of the operations.

```
time {
	load: <number> // Time to load an atom
	store: <number> // Time to store an atom
	ry: <number> // Time for ry-operation
	rz: <number> // Time for rz-operation
	cz: <number> // Time for cz-operation
	unit: <string> // Displayed time-unit
}
```

### Distances

The `distance`-block allows specifying various distances.

```
distance {
	interaction: <number> // Interaction-radius for all operations that operate on nearby atoms
	unit: <string> // Displayed distance-unit
}
```

### Zones

A zone can be defined with a `zone`-block.
It should be given a unique ID and bounds.

```
zone <id> {
	from: <position> // First coordinate of rectangle
	to: <position> // Second coordinate of rectangle
}
```

### Static Traps

A static trap can be defined with the `trap`-block.
It should be given a position.

```
trap <id> {
	position: <position> // Position of the trap
}
```

## Visual Configuration

Visual configuration can be specified in a `.nastyle`-file.
It contains the following blocks:

### Display-name

A human-readable name for the style can be set in the `name`-field.

```
name: <string> // Name of the style
```

### Atoms

The `atom`-block allows specifying settings regarding the appearance of atoms.

```
atom {
	trapped {
		color: <color> // Color of a trapped atom
	}
	shuttling {
		color: <color> // Color of a shuttling atom
	}
	legend {
		name {
			<regex>: <string> // Display a text over all atoms whose ID matches the key-regex; the displayed text is the replacement from the value
			// Example to display all ids: `^.*$: "$0"`
		}
		font {
			family: <string> // Font-Family of the text on the atoms
			size: <number> // Size of the text on the atoms
			color: <color> // Color of the text on the atoms
		}
	}
	radius: <number> // Radius of atoms
}
```

### Zones

The `zone`-block allows specifying settings regarding the appearance of zones.

```
zone {
	config <regex> { // The settings below will be applied to all zones matching this regex
		color: <color> // The color of the zone
		line {
			thickness: <number> // The line thickness of the zone
			dash {
				length: <number> // The length of dash-segments of the line
				duty: <percentage> // How much of the dash-segment will be filled
			}
		}
		name: <string> // What to display the zone as; the name can reference groups from the regex
	}
	legend {
		display: <boolean> // Whether to display the zone-names in the sidebar legend
		title: <string> // The heading over the zones in the sidebar
	}
}
```

### Operations

The `operation`-block allows specifying settings regarding the appearance of operations.

```
operation {
	config {
		ry {
			color: <color> // Color of ry-operations
			name: <string> // Name to display in the sidebar legend
			radius: <number | percentage> // How big the atoms should be during ry-operations; either absolute or relative
		}
		rz {
			color: <color> // Color of rz-operations
			name: <string> // Name to display in the sidebar legend
			radius: <number | percentage> // How big the atoms should be during rz-operations; either absolute or relative
		}
		cz {
			color: <color> // Color of cz-operations
			name: <string> // Name to display in the sidebar legend
			radius: <number | percentage> // How big the atoms should be during cz-operations; either absolute or relative
		}
	}
	legend {
		display: <boolean> // Whether to display the operation-names in the sidebar legend
		title: <string> // The heading over the operations in the sidebar
	}
}
```

### Machine

The `machine`-block allows specifying settings regarding the appearance of the machine.

```
machine {
	trap {
		color: <color> // Color of the traps
		radius: <number> // Radius of the traps
		line_width: <number> // Line width of the traps
		name: <string> // Name to display in the sidebar legend
	}
	shuttle {
		color: <color> // Color of the shuttle
		line {
			thickness: <number> // The line thickness of the shuttle
			dash {
				length: <number> // The length of dash-segments of the line
				duty: <percentage> // How much of the dash-segment will be filled
			}
		}
		name: <string> // Name to display in the sidebar legend
	}
	legend {
		display: <boolean> // Whether to display the trap and shuttle names in the sidebar legend
		title: <string> // The heading over the names in the sidebar
	}
}
```

### Coordinates

The `coordinate`-block allows specifying settings regarding the appearance of the coordinate system.

```
coordinate {
	tick {
		x: <number> // Distance of ticks in the x-direction
		y: <number> // Distance of ticks in the y-direction
		color: <color> // Color of the grid
		line {
			thickness: <number> // The line thickness of the grid
			dash {
				length: <number> // The length of dash-segments of the line
				duty: <percentage> // How much of the dash-segment will be filled
			}
		}
		display: <boolean> // Whether to display the coordinate ticks
	}
	number {
		x {
			distance: <number> // Distance between coordinate numbers in x-direction
			position: <'top' | 'bottom'> // Display numbers on top or bottom
		}
		y {
			distance: <number> // Distance between coordinate numbers in y-direction
			position: <'left' | 'right'> // Display numbers on left or right side
		}
		display: <boolean> // Whether to display the numbers
		font {
			family: <string> // Font-Family of the numbers
			size: <number> // Size of the numbers
			color: <color> // Color of the numbers
		}
	}
	axis {
		x: <string> // Name of the x-axis
		y: <string> // Name of the y-axis
		display: <boolean> // Whether to display the axis-names
		font {
			family: <string> // Font-Family of the axis names
			size: <number> // Size of the axis names
			color: <color> // Color of the axis names
		}
	}
	margin: <number> // Margins around the coordinate system; The content is automatically fitted to the smallest bounding rectangle which contains all zones and atoms at all times.
}
```

### Sidebar

The `sidebar`-block allows specifying settings regarding the appearance of the sidebar legend.

```
sidebar {
	font {
		family: <string> // Font-Family of the sidebar legend
		size: <number> // Size of the sidebar legend
		color: <color> // Color of the sidebar legend
	}
	margin: <number> // Margin around the sidebar legend
	padding {
		color: <number> // Distance between color display and text
		heading: <number> // Distance to advance for each header item; should be larger than font size
		entry: <number> // Distance to advance for each entry item; should be larger than font size
	}
	color_radius: <number> // The radius of the color display
}
```

### Time

The `time`-block allows specifying settings regarding the appearance of the time.

```
time {
	display: <boolean> // Whether to display the current time
	prefix: <string> // Text to display before the time
	precision: <number> // Number of decimal places to show. Should be a natural number.
	font {
		family: <string> // Font-Family of the time
		size: <number> // Size of the time
		color: <color> // Color of the time
	}
}
```

### Viewport

The `viewport`-block allows specifying settings regarding the appearance of the animation.

```
viewport {
	margin: <number> // Margin around the viewport
	color: <color> // Background-color of the viewport
}
```

## Visualization Input

### Properties

Some properties can be set using special directives.
These directives start with the `#`-character.

#### Target machines

Possible target machines can be specified using the `target`-directive.
Multiple machines may be specified using multiple `target`-directives.
This allows the user to select from the supported machines, though they may still force unsupported/unspecified machines.

```
#target <id>
#target <id>
#target <id>
```

### Instructions

#### Atoms

An atom can be defined with the `atom`-instruction.
It should be given a unique ID and a starting position.

```
atom <position> <id>
```

### Timed Instructions

Some instructions are timed, meaning they start at a specified time.
This starting-time is specified after an `@`-character at the start of the line.

```
@<time> <instruction>
```

#### Loading an atom

An atom can be loaded at its current position using the `load`-command, optionally with a load target position.

```
@<time> load [position] <id>
```

#### Storing an atom

An atom can be loaded at its current position using the `store`-command, optionally with a load target position.

```
@<time> store [position] <id>
```

#### Moving an atom

An atom can be moved to a new position using the `move`-command.

```
@<time> move <position> <id>
```

#### `rz`-operation

The `rz`-operation can be applied to a target using the `rz`-command.

```
@<time> rz <number> <target>
```

#### `ry`-operation

The `ry`-operation can be applied to a target using the `ry`-command.

```
@<time> ry <number> <target>
```

#### `cz`-operation

The `cz`-operation can be applied to a target using the `cz`-command.

```
@<time> cz <target>
```

### Syntactic Sugar

#### Automatic Time / Relative Time

The time may be specified relative to the start or the end of the previous operation.

- `@+` / `@+0` / `@-` / `@-0`: Execute immediately after end of preceding instruction
- `@+n`: Execute `n` time-steps after end of preceding instruction
- `@-n`: Execute `n` time-steps before end of preceding instruction
- `@=` / `@=+` / `@=+0` / `@=-` / `@=-0`: Execute immediately at start of preceding instruction
- `@=+n`: Execute `n` time-steps after start of preceding instruction
- `@=-n`: Execute `n` time-steps before start of preceding instruction

#### Grouping

Instructions and times may be grouped by specifying the time/instruction and all group-members in brackets (`[`/`]`).
A group's end-time (for the relative time of the next instruction) is the latest any of its instructions is finished.

Groups may be variable.
This is denoted by prepending a `~` to the group start symbol (`~[`/`]`).
Normal groups make all instructions take the same time.
A variable group makes all instructions take their minimum time.
The group's ent-time is not affected by this, only the end-times of its children instructions.

```
@<time> [
	<instruction>
	<instruction>
	<...>
	<instruction>
]

@<time> <instruction> [
	<arguments>
	<arguments>
	<...>
	<arguments>
]

// Variable groups

@<time> ~[
	<instruction>
	<instruction>
	<...>
	<instruction>
]

@<time> <instruction> ~[
	<arguments>
	<arguments>
	<...>
	<arguments>
]
```

## Comments

Comments in files are ignored.
They can be used to document the code or to temporarily ignore parts of the instructions.

### Single-line comments

Single-line comments start at `//` and go to the end of the line.

```
// This is a comment
atom (0, 0) atom0 // This is also a comment, but the instruction is executed
// atom (0, 0) atom0 // The instruction is commented out and ignored
```

### Multi-line comments

Multi-line comments start at `/*` and go to the next `*/`.

```
/*

This is a comment

This is still a comment

*/

/*
 * This is also a comment
 */

atom /* This is an inline comment */ (0, 0) atom0
```

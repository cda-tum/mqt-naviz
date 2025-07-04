# Usage

## Standalone Application

NAViz allows opening `.naviz` instruction files or importing instructions from external formats such as `mqt na` under the `File` menu.
Alternatively, files can simply be dropped onto or pasted into the application.
You can read more about the supported file formats in the {doc}`file format documentation <file_format>`.

The machine and style can be selected from the `Machine` and `Style` menus respectively.
These menus allow selecting a config from the loaded configs as well as opening or importing a new config.

When the animation plays, the progress bar at the bottom of the window can be used to seek through the visualization.

## Python Package

The python library currently only exports a simple functionality to export a visualization as a video.
An example can be seen below:

```python
from naviz import *

# Get machine and style from repository
machine = Repository.machines().get("example")
style = Repository.machines().get("tum")

# Alternatively, you can also use manual configurations
machine = "<...>"
style = "<...>"

# Render `naviz` instructions to `out.mp4` at 1080p60
export_video("<naviz instructions>", "out.mp4", (1920, 1080), 60, machine, style)

# Render `mqt na` instructions to `out.mp4` at 1080p60 with the default import options
# Alternatively, substitute the call to `default_import_settings` with your custom import settings
export_video(
    "<mqt na instructions>",
    "out.mp4",
    (1920, 1080),
    60,
    machine,
    style,
    default_import_settings("MqtNa"),
)
```

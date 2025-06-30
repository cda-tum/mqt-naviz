<!-- [![PyPI](https://img.shields.io/pypi/v/mqt.naviz?logo=pypi&style=flat-square)](https://pypi.org/project/mqt.naviz/) -->

![OS](https://img.shields.io/badge/os-linux%20%7C%20macos%20%7C%20windows-blue?style=flat-square)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)](https://opensource.org/licenses/MIT)
[![CI](https://img.shields.io/github/actions/workflow/status/cda-tum/mqt-naviz/ci.yml?branch=main&style=flat-square&logo=github&label=ci)](https://github.com/cda-tum/mqt-naviz/actions/workflows/ci.yml)
[![CD](https://img.shields.io/github/actions/workflow/status/cda-tum/mqt-naviz/cd.yml?style=flat-square&logo=github&label=cd)](https://github.com/cda-tum/mqt-naviz/actions/workflows/cd.yml)
[![Documentation](https://img.shields.io/readthedocs/mqt-naviz?logo=readthedocs&style=flat-square)](https://mqt.readthedocs.io/projects/naviz)
[![codecov](https://img.shields.io/codecov/c/github/cda-tum/mqt-naviz?style=flat-square&logo=codecov)](https://codecov.io/gh/cda-tum/mqt-naviz)

<p align="center">
  <a href="https://mqt.readthedocs.io">
   <picture>
     <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/munich-quantum-toolkit/.github/refs/heads/main/docs/_static/logo-mqt-dark.svg" width="60%">
     <img src="https://raw.githubusercontent.com/munich-quantum-toolkit/.github/refs/heads/main/docs/_static/logo-mqt-light.svg" width="60%" alt="MQT Logo">
   </picture>
  </a>
</p>

# MQT NAViz - A Visualizer for Neutral Atom Quantum Computers

MQT NAViz is an open-source Rust and Python library to visualize atom movements of neutral atom quantum computers.
It is developed as part of the [_Munich Quantum Toolkit (MQT)_](https://mqt.readthedocs.io).

<p align="center">
  <a href="https://mqt.readthedocs.io/projects/naviz">
  <img width=30% src="https://img.shields.io/badge/documentation-blue?style=for-the-badge&logo=read%20the%20docs" alt="Documentation" />
  </a>
</p>

## Key Features

- Instant playback of the loaded input neutral atom quantum computation
- Export of the visualization as a video
- Scrubbable timeline to navigate through the visualization
- Fully customizable machine architecture specification
- Support for different input formats
- Fully customizable visualization style and clean predefined styles

If you have any questions, feel free to create a [discussion](https://github.com/cda-tum/mqt-naviz/discussions) or an [issue](https://github.com/cda-tum/mqt-naviz/issues) on [GitHub](https://github.com/cda-tum/mqt-naviz).

## Contributors and Supporters

The _[Munich Quantum Toolkit (MQT)](https://mqt.readthedocs.io)_ is developed by the [Chair for Design Automation](https://www.cda.cit.tum.de/) at the [Technical University of Munich](https://www.tum.de/) and supported by the [Munich Quantum Software Company (MQSC)](https://munichquantum.software).
Among others, it is part of the [Munich Quantum Software Stack (MQSS)](https://www.munich-quantum-valley.de/research/research-areas/mqss) ecosystem, which is being developed as part of the [Munich Quantum Valley (MQV)](https://www.munich-quantum-valley.de) initiative.

<p align="center">
  <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/munich-quantum-toolkit/.github/refs/heads/main/docs/_static/mqt-logo-banner-dark.svg" width="90%">
   <img src="https://raw.githubusercontent.com/munich-quantum-toolkit/.github/refs/heads/main/docs/_static/mqt-logo-banner-light.svg" width="90%" alt="MQT Partner Logos">
  </picture>
</p>

Thank you to all the contributors who have helped make MQT NAViz a reality!

<p align="center">
<a href="https://github.com/cda-tum/mqt-naviz/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=cda-tum/mqt-naviz" />
</a>
</p>

## Getting Started

`mqt.naviz` is available via [PyPI](https://pypi.org/project/mqt.naviz/) for all major operating systems and supports Python 3.9 to 3.13.

```console
(.venv) $ pip install mqt.naviz
```

The following code gives an example of the usage:

<!-- todo: Fill in example code below -->

```python3
from mqt.naviz import Repository, export_video
```

**Detailed documentation and examples are available at [ReadTheDocs](https://mqt.readthedocs.io/projects/naviz).**

## System Requirements

Building (and running) is continuously tested under Linux, MacOS, and Windows using the [latest available system versions for GitHub Actions](https://github.com/actions/runner-images).
However, the implementation should be compatible with any current Rust compiler.

MQT NAViz relies on some external dependencies:

<!-- todo: Add deps as bullet points with links -->

---

## Acknowledgements

The Munich Quantum Toolkit has been supported by the European
Research Council (ERC) under the European Union's Horizon 2020 research and innovation program (grant agreement
No. 101001318), the Bavarian State Ministry for Science and Arts through the Distinguished Professorship Program, as well as the
Munich Quantum Valley, which is supported by the Bavarian state government with funds from the Hightech Agenda Bayern Plus.

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/munich-quantum-toolkit/.github/refs/heads/main/docs/_static/mqt-funding-footer-dark.svg" width="90%">
    <img src="https://raw.githubusercontent.com/munich-quantum-toolkit/.github/refs/heads/main/docs/_static/mqt-funding-footer-light.svg" width="90%" alt="MQT Funding Footer">
  </picture>
</p>

# MQT NAViz - A Visualizer for Neutral Atom Quantum Computers

```{raw} latex
\begin{abstract}
```

MQT NAViz is an open-source Rust and Python library to visualize atom movements of neutral atom quantum computers.
It is developed as part of the _{doc}`Munich Quantum Toolkit (MQT) <mqt:index>`_ by the [Chair for Design Automation](https://www.cda.cit.tum.de/) at the [Technical University of Munich](https://www.tum.de).

This documentation provides a comprehensive guide to the MQT NAViz library, including {doc}`installation instructions <installation>`, demo notebooks, and detailed {doc}`API documentation <api/mqt/naviz/index>`.
The source code of MQT NAViz is publicly available on GitHub at [cda-tum/mqt-naviz](https://github.com/cda-tum/mqt-naviz), while pre-built binaries are available via [PyPI](https://pypi.org/project/mqt.naviz/) for all major operating systems and all modern Python versions.

We recommend you to start with the {doc}`installation instructions <installation>`.
Then proceed to the {doc}`usage page <usage>` or {doc}`file format page <file_format>` and read the {doc}`reference documentation <api/mqt/naviz/index>`.

We appreciate any feedback and contributions to the project. If you want to contribute, you can find more information in the {doc}`contribution guide <contributing>`.
If you are having trouble with the installation or the usage of NAViz, please let us know at our {doc}`support page <support>` or by reaching out to us at [quantum.cda@xcit.tum.de](mailto:quantum.cda@xcit.tum.de).

````{only} latex
```{note}
A live version of this document is available at [mqt.readthedocs.io/projects/naviz](https://mqt.readthedocs.io/projects/naviz).
```
````

```{raw} latex
\end{abstract}

\sphinxtableofcontents
```

```{toctree}
:hidden:

self
```

```{toctree}
:maxdepth: 2
:caption: User Guide

installation
usage
CHANGELOG
UPGRADING
```

````{only} not latex
```{toctree}
:maxdepth: 2
:titlesonly:
:caption: Developers
:glob:

contributing
support
development_guide
```
````

```{toctree}
:hidden:
:maxdepth: 6
:caption: API Reference

api/mqt/naviz/index
crates/animator/lib
crates/bindings/lib
crates/renderer/lib
crates/repository/lib
crates/state/lib
crates/video/lib
```

```{only} html
## Contributors and Supporters

The _[Munich Quantum Toolkit (MQT)](https://mqt.readthedocs.io)_ is developed by the [Chair for Design Automation](https://www.cda.cit.tum.de/) at the [Technical University of Munich](https://www.tum.de/) and supported by the [Munich Quantum Software Company (MQSC)](https://munichquantum.software).
Among others, it is part of the [Munich Quantum Software Stack (MQSS)](https://www.munich-quantum-valley.de/research/research-areas/mqss) ecosystem, which is being developed as part of the [Munich Quantum Valley (MQV)](https://www.munich-quantum-valley.de) initiative.

<div style="margin-top: 0.5em">
<div class="only-light" align="center">
  <img src="https://raw.githubusercontent.com/munich-quantum-toolkit/.github/refs/heads/main/docs/_static/mqt-logo-banner-light.svg" width="90%" alt="MQT Banner">
</div>
<div class="only-dark" align="center">
  <img src="https://raw.githubusercontent.com/munich-quantum-toolkit/.github/refs/heads/main/docs/_static/mqt-logo-banner-dark.svg" width="90%" alt="MQT Banner">
</div>
</div>

Thank you to all the contributors who have helped make MQT NAViz a reality!

<p align="center">
<a href="https://github.com/cda-tum/mqt-naviz/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=cda-tum/mqt-naviz" />
</a>
</p>
```

# App development model

One of the guiding principles for this project is that _common things should be easy and rare things should be possible_.
To help make common things easy and to communicate how the tools are designed to work it is helpful crate some models.

## Typical workflow

```mermaid
flowchart LR
    subgraph zero [setup]
        direction TB
        checkout[git checkout]
        checkout --> reinit[device-manager reinit]
        reinit --> install[cargo-acap-sdk install]
    end

    zero --> edit[\edit/] --> one

    subgraph one [host tests]
        direction TB
        utest[cargo test]
    end

    one -- success? --> two

    subgraph two [target tests]
        direction TB
        test[cargo-acap-sdk test]
    end

    two -- success? --> three

    subgraph three [external tests]
        direction TB
        run[cargo-acap-sdk run] --> atest[automatic tests]
        atest --> mtest[\manual tests/]
    end

    three -- success? --> four

    subgraph four [validation]
        direction TB
        commit[\git commit/] --> remove[cargo-acap-sdk remove]
        remove --> install4[cargo-acap-sdk install]
        install4 --> start[cargo-acap-sdk start]
        start --> atest4[automatic tests]
        atest4 --> mtest4[\manual tests/]
    end

    four -- success? --> push[git push]
```

If any step fails, the solution is typically going back to edit the code. 

Note that this is only a model and as such it is necessarily wrong, but hopefully this one is
useful.

## Device state

Another way to understand the development process and tools is by looking at how the state of the
device is affected by various commands:

```mermaid
stateDiagram-v2
    [*] --> baseline: reinit
    baseline --> installed: install
    installed --> stopped: run
    installed --> stopped: test
    installed --> started: start
    stopped --> stopped: run
    stopped --> stopped: test
    stopped --> started: start
    stopped --> removed: remove
    removed --> installed: install
    started --> removed: remove
    started --> stopped: stop
```

Notable aspects omitted from the above model include:

- Build profile: `install` builds the app with a different profile than `run` and `test`.
- Rare side effects: installing and running an app can have side effects that are not undone by removing the app.
  This is especially true on older firmwares where the post-install script and the app itself were allowed to run as root.
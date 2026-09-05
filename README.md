# GlobalChat

Not very long ago right before your time: Famine, Murder, the world was crazy, every [Smartschool++](https://smartschoolplusplus.com) user wanted a place where they could chat with each other, Where they could share their (not racist) opinions with all that would listen and where they could be who they really are. _And then.._, It happened. The ground shook with the force of ten earthquakes as oceans of people rushed towards the light emerging from the horizon getting closer and closer. The Entire world stopped as the swarm of people arrived. They all looked up in aw at the entity that had landed on their earth. _The Garbage Collector_, ehum I mean _[Global Chat](https://gc.smartschoolplusplus.com)_. Finally the world was free, the savior everyone needed had arrived. All the long days of endless suffering, It was all over and the world could slowly be restored to peace again. But between the chaos of the crowed some entity, some _object_? had gone missing. Some would have even classified it as _[real](https://github.com/sprksoft/realperson)_.

## Development setup

1. Install docker & docker compose this can be easily done by installing [docker desktop](https://docs.docker.com/desktop/setup/install/windows-install)
2. Install docker desktop and [just](https://github.com/casey/just)
3. Install the latest stable [rust compiler](https://rustup.rs)
4. Run `cargo install sqlx-cli` to install the cli sqlx tools.
5. (Optional) Install rust analyzer for autocomplete and nice features. If using vscode you can [download the rust analyzer extension](https://code.visualstudio.com/docs/languages/rust)
6. Run `just` in the project root and wait until everything has finished

> [!NOTE]
> If you make changes to the code you should use `just build` to force a rebuild

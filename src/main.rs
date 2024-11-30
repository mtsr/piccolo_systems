use bevy::{
    asset::{io::Reader, AssetLoader, AssetPath, LoadContext},
    ecs::system::{DynParamBuilder, DynSystemParam, ParamBuilder, SystemState},
    prelude::*,
};

use piccolo::{Closure, Executor, Lua};
use thiserror::Error;

fn main() {
    let mut app = App::default();
    app.add_plugins(DefaultPlugins.set(AssetPlugin {
        watch_for_changes_override: Some(true),
        ..default()
    }));

    app
        // .add_systems(Update, update_lua_script_loader)
        .add_systems(PreUpdate, update_lua_systems);

    app.init_asset::<LuaFile>();
    app.init_asset_loader::<LuaScriptLoader>();

    app.init_resource::<LuaSystems>();

    app.run();
}

#[derive(Debug, Resource)]
struct LuaSystems {
    systems: Vec<Handle<LuaFile>>,
}

impl FromWorld for LuaSystems {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let mut systems = Vec::new();
        systems.push(asset_server.load("test.lua"));
        LuaSystems { systems }
    }
}

fn update_lua_systems(world: &mut World) {
    let systems = {
        let mut systems: Vec<LuaFile> = Vec::new();

        let mut system_state: SystemState<(
            EventReader<AssetEvent<LuaFile>>,
            ResMut<Assets<LuaFile>>,
            NonSendMut<LuaVm>,
            ResMut<Schedules>,
        )> = SystemState::new(world);
        let (mut asset_events, lua_files, mut lua_vm, mut schedules) = system_state.get_mut(world);

        for event in asset_events.read() {
            match event {
                AssetEvent::Added { id } => {
                    if let Some(file) = lua_files.get(*id) {
                        println!("Added: {:?}", file);

                        systems.push(file.clone());
                    }
                }
                AssetEvent::Modified { id } => {
                    if let Some(file) = lua_files.get(*id) {
                        println!("Modified: {:?}", file);
                    }
                }
                AssetEvent::Removed { id } => {
                    if let Some(file) = lua_files.get(*id) {
                        println!("Removed: {:?}", file);
                    }
                }
                AssetEvent::Unused { id } => {
                    if let Some(file) = lua_files.get(*id) {
                        println!("Unused: {:?}", file);
                    }
                }
                AssetEvent::LoadedWithDependencies { id } => {
                    if let Some(file) = lua_files.get(*id) {
                        println!("LoadedWithDependencies: {:?}", file);
                    }
                }
            }
        }
        systems
    };

    let systems = systems
        .into_iter()
        .map(|system| {
            println!("System: {:?}", system);
            let bytes = system.bytes.clone();

            (
                ParamBuilder::of::<NonSendMut<LuaVm>>(),
                // DynParamBuilder::new(ParamBuilder::query::<&Camera>()),
            )
                // ()
                .build_state(world)
                .build_system(move |mut lua_vm| {
                    // .build_system(|| {
                    // println!("Hello from dynamic system");
                    let executor = lua_vm
                        .lua
                        .try_enter(|ctx| {
                            let closure = Closure::load(ctx, None, bytes.as_slice())?;
                            Ok(ctx.stash(Executor::start(ctx, closure.into(), ())))
                        })
                        .unwrap();
                    lua_vm.lua.execute::<()>(&executor).unwrap();
                })
        })
        .collect::<Vec<_>>();

    let mut schedules = world.get_resource_mut::<Schedules>().unwrap();
    for system in systems.into_iter() {
        println!("Adding system to schedule {:?}", system.name());
        schedules.entry(Update).add_systems(system);
    }
}

#[derive(Asset, Clone, TypePath, Debug)]
struct LuaFile {
    bytes: Vec<u8>,
}

struct LuaScriptLoader;

struct LuaVm {
    lua: Lua,
}

impl FromWorld for LuaScriptLoader {
    fn from_world(world: &mut World) -> Self {
        let lua = Lua::full();
        world.insert_non_send_resource(LuaVm { lua });

        LuaScriptLoader
    }
}

#[non_exhaustive]
#[derive(Debug, Error)]
enum LuaScriptLoaderError {
    #[error("Could not load file: {0}")]
    Io(#[from] std::io::Error),
    // LuaError(rlua::Error),
}

impl AssetLoader for LuaScriptLoader {
    type Asset = LuaFile;
    type Settings = ();
    type Error = LuaScriptLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(LuaFile { bytes })
    }

    fn extensions(&self) -> &[&str] {
        &["lua"]
    }
}

fn update_lua_script_loader(mut lua_vm: NonSendMut<LuaVm>) {
    let executor = lua_vm
        .lua
        .try_enter(|ctx| {
            let closure = Closure::load(
                ctx,
                None,
                ("print(\"Hello from lua\")".to_owned()).as_bytes(),
            )?;
            Ok(ctx.stash(Executor::start(ctx, closure.into(), ())))
        })
        .unwrap();

    lua_vm.lua.execute::<()>(&executor).unwrap();
}

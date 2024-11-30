use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    ecs::system::{ParamBuilder, SystemState},
    prelude::*,
    utils::HashSet,
};

use piccolo::{Closure, Executor, Lua};
use thiserror::Error;

fn main() {
    let mut app = App::default();
    app.add_plugins(DefaultPlugins.set(AssetPlugin {
        watch_for_changes_override: Some(true),
        ..default()
    }));

    app.add_systems(Startup, setup)
        .add_systems(PreUpdate, update_lua_systems);

    app.init_asset::<LuaFile>();
    app.init_asset_loader::<LuaScriptLoader>();

    let lua = Lua::full();
    app.insert_non_send_resource(LuaVm { lua });

    app.init_resource::<LuaFiles>();

    app.run();
}

#[derive(Debug, Default, Resource)]
struct LuaFiles {
    files: Vec<Handle<LuaFile>>,
}

fn setup(mut lua_files: ResMut<LuaFiles>, asset_server: Res<AssetServer>) {
    lua_files.files.push(asset_server.load("test.lua"));
}

fn update_lua_systems(world: &mut World) {
    // Build Vec of systems from LuaFiles
    // scoped because of world access
    let systems = {
        let mut systems: HashSet<AssetId<LuaFile>> = HashSet::new();

        // Use system state to access all required resources
        let mut system_state: SystemState<(EventReader<AssetEvent<LuaFile>>,)> =
            SystemState::new(world);

        let (mut asset_events,) = system_state.get_mut(world);

        for event in asset_events.read() {
            match event {
                AssetEvent::Added { id } => {
                    println!("Added: {:?}", id);
                    systems.insert(*id);
                }
                AssetEvent::Modified { id } => {
                    println!("Modified: {:?}", id);
                }
                AssetEvent::Removed { id } => {
                    println!("Removed: {:?}", id);
                }
                AssetEvent::Unused { id } => {
                    println!("Unused: {:?}", id);
                }
                AssetEvent::LoadedWithDependencies { id } => {
                    println!("LoadedWithDependencies: {:?}", id);
                }
            }
        }
        systems
    };

    let systems = systems
        .into_iter()
        .map(|id| {
            println!("System: {:?}", id);

            (
                ParamBuilder::of::<NonSendMut<LuaVm>>(),
                ParamBuilder::resource::<Assets<LuaFile>>(),
            )
                .build_state(world)
                .build_system(move |mut lua_vm, lua_files| {
                    let bytes = lua_files.get(id).unwrap().bytes.clone();
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
    fn from_world(_world: &mut World) -> Self {
        LuaScriptLoader
    }
}

#[non_exhaustive]
#[derive(Debug, Error)]
enum LuaScriptLoaderError {
    #[error("Could not load file: {0}")]
    Io(#[from] std::io::Error),
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

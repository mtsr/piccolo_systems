use bevy::{
    asset::{io::Reader, AssetLoader, AssetPath, LoadContext},
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

    app.add_systems(PreStartup, pre_startup)
        .add_systems(Update, update_lua_script_loader);

    app.init_asset::<LuaScript>();
    app.init_asset_loader::<LuaScriptLoader>();

    app.run();
}

fn pre_startup(mut schedules: ResMut<Schedules>, asset_server: Res<AssetServer>) {
    for (label, schedule) in schedules.iter() {
        println!("Schedule: {:?}", label);
    }

    let script: Handle<LuaScript> = asset_server.load("test.lua");
    println!("Script: {:?}", script);

    schedules.get_mut(Update).unwrap().add_systems(update);
}

fn update() {
    println!("Update");
}

#[derive(Asset, TypePath, Debug)]
struct LuaScript {}

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
    type Asset = LuaScript;
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
        // let lua_script = ron::de::from_bytes::<LuaScript>(&bytes)?;
        // Ok(lua_script)
        Ok(LuaScript {})
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

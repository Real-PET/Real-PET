use super::*;
use crate::bindable::BlockColor;
use crate::lua_api::create_analytical_vm;
use crate::resources::{LocalAssetManager, ResourcePaths};
use std::cell::RefCell;

#[derive(Default, Clone)]
pub struct BlockPackage {
    pub package_info: PackageInfo,
    pub name: String,
    pub description: String,
    pub is_program: bool,
    pub block_color: BlockColor,
    pub shape: [bool; 5 * 5],
}

impl Package for BlockPackage {
    fn package_info(&self) -> &PackageInfo {
        &self.package_info
    }

    fn package_info_mut(&mut self) -> &mut PackageInfo {
        &mut self.package_info
    }

    fn load_new(assets: &LocalAssetManager, package_info: PackageInfo) -> Self {
        let package = RefCell::new(BlockPackage::default());
        package.borrow_mut().package_info = package_info.clone();

        let lua = create_analytical_vm(assets, &package_info);

        let globals = lua.globals();
        let package_init: rollback_mlua::Function = match globals.get("package_init") {
            Ok(package_init) => package_init,
            _ => {
                log::error!(
                    "missing package_init() in {:?}",
                    ResourcePaths::shorten(&package_info.script_path)
                );
                return package.into_inner();
            }
        };

        let result = lua.scope(|scope| {
            crate::lua_api::analytical_api::inject_analytical_api(&lua, scope, assets, &package)?;
            crate::lua_api::analytical_api::query_dependencies(&lua);

            let package_table = lua.create_table()?;

            package_table.set(
                "declare_package_id",
                scope.create_function(|_, (_, id): (rollback_mlua::Table, String)| {
                    package.borrow_mut().package_info.id = id;
                    Ok(())
                })?,
            )?;

            package_table.set(
                "set_name",
                scope.create_function(|_, (_, name): (rollback_mlua::Table, String)| {
                    package.borrow_mut().name = name;

                    Ok(())
                })?,
            )?;

            package_table.set(
                "set_description",
                scope.create_function(|_, (_, path): (rollback_mlua::Table, String)| {
                    package.borrow_mut().description = package_info.base_path.to_string() + &path;
                    Ok(())
                })?,
            )?;

            package_table.set(
                "set_color",
                scope.create_function(|_, (_, block_color): (rollback_mlua::Table, u8)| {
                    use num_traits::FromPrimitive;

                    package.borrow_mut().block_color =
                        BlockColor::from_u8(block_color).unwrap_or_default();
                    Ok(())
                })?,
            )?;

            package_table.set(
                "set_shape",
                scope.create_function(|_, (_, mut bools): (rollback_mlua::Table, Vec<bool>)| {
                    bools.resize(5 * 5, false);

                    let mut package = package.borrow_mut();

                    for (i, bool) in bools.into_iter().enumerate() {
                        package.shape[i] = bool;
                    }

                    Ok(())
                })?,
            )?;

            // stub
            package_table.set("set_mutator", scope.create_function(|_, _: ()| Ok(()))?)?;

            package_init.call(package_table)?;

            Ok(())
        });

        if let Err(e) = result {
            log::error!("{e}");
        }

        package.into_inner()
    }
}

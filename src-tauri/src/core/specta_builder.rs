use crate::{
    context::{account, asset},
    core::{logger, Event},
    use_cases::update_checker,
};

/// create the Specta builder for standard and generate_bindings
pub fn create_specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .typ::<update_checker::UpdateInfo>()
        .typ::<asset::Asset>()
        .typ::<asset::AssetCategory>()
        .typ::<asset::AssetPrice>()
        .typ::<asset::AssetClass>()
        .typ::<account::Account>()
        .typ::<account::UpdateFrequency>()
        .commands(tauri_specta::collect_commands![
            asset::get_assets,
            asset::get_assets_with_archived,
            asset::add_asset,
            asset::update_asset,
            asset::archive_asset,
            asset::unarchive_asset,
            asset::delete_asset,
            asset::get_categories,
            asset::add_category,
            asset::update_category,
            asset::delete_category,
            asset::create_asset_price,
            account::get_accounts,
            account::add_account,
            account::update_account,
            account::delete_account,
            logger::log_frontend,
            update_checker::check_for_update,
            update_checker::download_update,
            update_checker::install_update
        ])
        .events(tauri_specta::collect_events![Event])
}

use crate::{
    context::{account, asset},
    core::{logger, Event},
};

/// create the Specta builder for standard and generate_bindings
pub fn create_specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .typ::<asset::Asset>()
        .typ::<asset::AssetCategory>()
        .typ::<asset::AssetPrice>()
        .typ::<asset::AssetClass>()
        .typ::<account::Account>()
        .typ::<account::AssetAccount>()
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
            account::get_account_holdings,
            account::upsert_account_holding,
            account::remove_account_holding,
            logger::log_frontend
        ])
        .events(tauri_specta::collect_events![Event])
}

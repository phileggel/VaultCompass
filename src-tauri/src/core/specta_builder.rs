use crate::{
    context::{account, asset, transaction},
    core::{logger, Event},
    use_cases::{account_details, record_transaction, update_checker},
};

/// create the Specta builder for standard and generate_bindings
pub fn create_specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .typ::<update_checker::UpdateInfo>()
        .typ::<asset::Asset>()
        .typ::<asset::AssetCategory>()
        .typ::<asset::AssetClass>()
        .typ::<account::Account>()
        .typ::<account::UpdateFrequency>()
        .typ::<account::Holding>()
        .typ::<transaction::Transaction>()
        .typ::<transaction::TransactionType>()
        .typ::<record_transaction::CreateTransactionDTO>()
        .typ::<account_details::HoldingDetail>()
        .typ::<account_details::ClosedHoldingDetail>()
        .typ::<account_details::AccountDetailsResponse>()
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
            asset::record_asset_price,
            account::get_accounts,
            account::add_account,
            account::update_account,
            account::delete_account,
            logger::log_frontend,
            update_checker::check_for_update,
            update_checker::download_update,
            update_checker::install_update,
            record_transaction::add_transaction,
            record_transaction::update_transaction,
            record_transaction::delete_transaction,
            record_transaction::get_transactions,
            transaction::get_asset_ids_for_account,
            account_details::get_account_details
        ])
        .events(tauri_specta::collect_events![Event])
}

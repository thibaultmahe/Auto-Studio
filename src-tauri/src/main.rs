#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod secrets;
use serde::Serialize;
use std::sync::Arc;
use studio_core::{provider::InferenceSh, *};
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::Semaphore;

struct Studio {
    library: Library,
    gate: Arc<Semaphore>,
}
type UiResult<T> = std::result::Result<T, String>;
async fn heavy<T, F>(state: State<'_, Studio>, f: F) -> UiResult<T>
where
    T: Send + 'static,
    F: FnOnce(Library) -> UiResult<T> + Send + 'static,
{
    let lib = state.library.clone();
    let permit = state
        .gate
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| "L’atelier se ferme.".to_owned())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        f(lib)
    })
    .await
    .map_err(|_| "L’opération locale a été interrompue.".to_owned())?
}
fn ui<T>(result: studio_core::Result<T>) -> UiResult<T> {
    result.map_err(|e| e.to_string())
}
#[derive(Serialize)]
struct Snapshot {
    photos: Vec<Photo>,
    settings: Settings,
    submissions: Vec<Submission>,
    library_location: String,
    windows: bool,
}
#[tauri::command]
async fn snapshot(state: State<'_, Studio>) -> UiResult<Snapshot> {
    let lib = state.library.clone();
    tauri::async_runtime::spawn_blocking(move || {
        Ok(Snapshot {
            photos: ui(lib.photos())?,
            settings: ui(lib.settings())?,
            submissions: ui(lib.submissions())?,
            library_location: lib.root().display().to_string(),
            windows: cfg!(windows),
        })
    })
    .await
    .map_err(|_| "Lecture interrompue.".to_owned())?
}
#[tauri::command]
async fn import_photo(app: tauri::AppHandle, state: State<'_, Studio>) -> UiResult<Option<Photo>> {
    heavy(state, move |lib| {
        let Some(file) = app
            .dialog()
            .file()
            .set_title("Importer une photo")
            .add_filter("Photos", &["jpg", "jpeg", "png", "webp"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        let path = file
            .into_path()
            .map_err(|_| "Choisissez un fichier local.".to_owned())?;
        ui(lib.import(&path)).map(Some)
    })
    .await
}
#[tauri::command]
async fn classify(state: State<'_, Studio>, id: String, category: Category) -> UiResult<Photo> {
    heavy(state, move |l| ui(l.classify(&id, category))).await
}
#[tauri::command]
async fn preview(
    state: State<'_, Studio>,
    id: String,
    kind: Asset,
    thumbnail: bool,
) -> UiResult<String> {
    heavy(state, move |l| ui(l.preview(&id, kind, thumbnail))).await
}
#[tauri::command]
async fn plan(state: State<'_, Studio>, id: String) -> UiResult<Plan> {
    heavy(state, move |l| ui(l.plan(&id))).await
}
#[tauri::command]
async fn submit_photo(
    state: State<'_, Studio>,
    id: String,
    expected_version: String,
    confirmed: bool,
) -> UiResult<Photo> {
    if !confirmed {
        return Err("Confirmez explicitement l’envoi au fournisseur.".into());
    }
    heavy(state, move |l| {
        let key = secrets::get()?;
        ui(l.submit(&id, &expected_version, &ui(InferenceSh::new())?, &key))
    })
    .await
}
#[tauri::command]
async fn sync_photo(
    state: State<'_, Studio>,
    id: String,
    remote_id: Option<String>,
) -> UiResult<Photo> {
    heavy(state, move |l| {
        let key = secrets::get()?;
        ui(l.synchronize(&id, remote_id.as_deref(), &ui(InferenceSh::new())?, &key))
    })
    .await
}
#[tauri::command]
async fn compose_photo(
    state: State<'_, Studio>,
    id: String,
    settings: Composition,
) -> UiResult<Photo> {
    heavy(state, move |l| ui(l.compose(&id, &settings))).await
}
#[tauri::command]
async fn review_photo(state: State<'_, Studio>, id: String, approved: bool) -> UiResult<Photo> {
    heavy(state, move |l| ui(l.review(&id, approved))).await
}
#[tauri::command]
async fn export_photo(
    app: tauri::AppHandle,
    state: State<'_, Studio>,
    id: String,
    format: ExportFormat,
    quality: u8,
) -> UiResult<Option<String>> {
    heavy(state, move |l| {
        let Some(folder) = app
            .dialog()
            .file()
            .set_title("Dossier d’export")
            .blocking_pick_folder()
        else {
            return Ok(None);
        };
        let folder = folder
            .into_path()
            .map_err(|_| "Choisissez un dossier local.".to_owned())?;
        ui(l.export(&id, &folder, format, quality)).map(|p| Some(p.display().to_string()))
    })
    .await
}
#[tauri::command]
async fn save_limits(state: State<'_, Studio>, limits: Limits) -> UiResult<()> {
    heavy(state, move |l| {
        let mut s = ui(l.settings())?;
        s.limits = limits;
        ui(l.save_settings(&s))
    })
    .await
}
#[tauri::command]
async fn key_status() -> UiResult<bool> {
    tauri::async_runtime::spawn_blocking(secrets::present)
        .await
        .map_err(|_| "Consultation interrompue.".to_owned())?
}
#[tauri::command]
async fn save_key(key: String) -> UiResult<()> {
    tauri::async_runtime::spawn_blocking(move || secrets::save(key))
        .await
        .map_err(|_| "Enregistrement interrompu.".to_owned())?
}
#[tauri::command]
async fn delete_key() -> UiResult<()> {
    tauri::async_runtime::spawn_blocking(secrets::delete)
        .await
        .map_err(|_| "Suppression interrompue.".to_owned())?
}
#[tauri::command]
async fn pin_provider(state: State<'_, Studio>) -> UiResult<ProviderPin> {
    heavy(state, move |l| {
        let key = secrets::get()?;
        ui(l.pin_provider(&ui(InferenceSh::new())?, &key))
    })
    .await
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let path = app.path().app_local_data_dir()?.join("phototheque");
            let lib = Library::open(path)?;
            app.manage(Studio {
                library: lib,
                gate: Arc::new(Semaphore::new(1)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            import_photo,
            classify,
            preview,
            plan,
            submit_photo,
            sync_photo,
            compose_photo,
            review_photo,
            export_photo,
            save_limits,
            key_status,
            save_key,
            delete_key,
            pin_provider
        ])
        .run(tauri::generate_context!())
        .expect("Impossible de démarrer AutoStudio Desktop");
}

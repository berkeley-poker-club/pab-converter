use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Event, FileReader, HtmlInputElement, DragEvent};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Serialize, Deserialize)]
struct ConvertArgs {
    content: String,
}

#[derive(Clone, Default)]
struct FileInfo {
    name: String,
    size: f64,
}

#[component]
pub fn App() -> impl IntoView {
    let (is_dragging, set_is_dragging) = signal(false);
    let (selected_file_info, set_selected_file_info) = signal::<Option<FileInfo>>(None);
    let (upload_status, set_upload_status) = signal::<Option<String>>(None);
    let (is_converting, set_is_converting) = signal(false);
    let (converted_content, set_converted_content) = signal(String::new());

    let file_input_ref = NodeRef::<leptos::html::Input>::new();

    let handle_files = move |files: web_sys::FileList| {
        if files.length() > 0 {
            if let Some(file) = files.item(0) {
                let file_name = file.name();
                if !file_name.ends_with(".ohh")
                    && !file_name.ends_with(".txt")
                    && !file_name.ends_with(".json")
                {
                    set_upload_status.set(Some(
                        "Error: Please upload an OHH, TXT, or JSON file".to_string(),
                    ));
                    set_selected_file_info.set(None);
                    return;
                }

                let file_info = FileInfo {
                    name: file_name.clone(),
                    size: file.size(),
                };

                set_selected_file_info.set(Some(file_info));
                set_upload_status.set(Some(format!("Selected: {}", file_name)));

                // Start reading the file
                match FileReader::new() {
                    Ok(file_reader) => {
                        let fr = file_reader.clone();

                        let onload = Closure::wrap(Box::new(move |_: Event| {
                            if let Ok(result) = fr.result() {
                                if let Some(text) = result.as_string() {
                                    set_upload_status.set(Some("File ready to convert".to_string()));
                                }
                            }
                        }) as Box<dyn FnMut(_)>);

                        file_reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                        onload.forget();

                        let _ = file_reader.read_as_text(&file);
                    }
                    Err(_) => {
                        set_upload_status.set(Some(
                            "Error: Failed to initialize file reader".to_string(),
                        ));
                    }
                }
            }
        }
    };

    let on_file_input = move |ev: Event| {
        if let Some(target) = ev.target() {
            if let Ok(input) = target.dyn_into::<HtmlInputElement>() {
                if let Some(files) = input.files() {
                    handle_files(files);
                }
            }
        }
    };

    // Drag & drop handlers (defined after handle_files so they can capture it)
    let on_drag_over = move |ev: DragEvent| {
        ev.prevent_default();
        set_is_dragging.set(true);
    };

    let on_drag_leave = move |ev: DragEvent| {
        ev.prevent_default();
        set_is_dragging.set(false);
    };

    let on_drop = move |ev: DragEvent| {
        ev.prevent_default();
        if let Some(dt) = ev.data_transfer() {
            if let Some(files) = dt.files() {
                handle_files(files);
            }
        }
        set_is_dragging.set(false);
    };

    let convert_file = move |_| {
        if let Some(input_element) = file_input_ref.get() {
            let input_el: &HtmlInputElement = input_element.as_ref();
            if let Some(files) = input_el.files() {
                if let Some(file) = files.item(0) {
                    let file_name = file.name().clone();
                    set_is_converting.set(true);
                    set_upload_status.set(Some("Converting...".to_string()));

                    // Clone signals for use in the async block
                    let set_is_converting_clone = set_is_converting;
                    let set_upload_status_clone = set_upload_status;
                    let set_converted_content_clone = set_converted_content;
                    let set_selected_file_info_clone = set_selected_file_info;

                    spawn_local(async move {
                        // Use web_sys Blob reader to read the file
                        match wasm_bindgen_futures::JsFuture::from(file.slice().unwrap().text()).await
                        {
                            Ok(text_promise) => {
                                if let Some(content) = text_promise.as_string() {
                                    match serde_wasm_bindgen::to_value(&ConvertArgs { content }) {
                                        Ok(args) => {
                                            match invoke::<_, String>("convert_ohh_content", args).await {
                                                Ok(_) => {
                                                    set_upload_status_clone.set(Some("conversion completed successfully.".to_string()));
                                                    set_is_converting_clone.set(false);
                                                }
                                                Err(e) => {
                                                    set_upload_status_clone.set(Some(format!("error: {}", e)));
                                                    set_is_converting_clone.set(false);
                                                }
                                            }


                                            if let Some(response) = result.as_string() {
                                                if response.starts_with("Error")
                                                    || response.starts_with("Failed")
                                                {
                                                    set_upload_status_clone.set(Some(format!(
                                                        "[ERR] {}",
                                                        response
                                                    )));
                                                    set_converted_content_clone.set(String::new());
                                                } else {
                                                    set_converted_content_clone.set(response);
                                                    set_upload_status_clone.set(Some(format!(
                                                        "[OK] Successfully converted: {}",
                                                        file_name
                                                    )));
                                                    set_selected_file_info_clone.set(None);
                                                }
                                            } else {
                                                set_upload_status_clone.set(Some(
                                                    "[ERR] Invalid response from backend".to_string(),
                                                ));
                                            }
                                        }
                                        Err(e) => {
                                            set_is_converting_clone.set(false);
                                            set_upload_status_clone.set(Some(format!(
                                                "[ERR] Failed to prepare request: {}",
                                                e
                                            )));
                                        }
                                    }
                                } else {
                                    set_is_converting_clone.set(false);
                                    set_upload_status_clone.set(Some(
                                        "[ERR] Could not read file content".to_string(),
                                    ));
                                }
                            }
                            Err(_) => {
                                set_is_converting_clone.set(false);
                                set_upload_status_clone.set(Some(
                                    "[ERR] Failed to read file".to_string(),
                                ));
                            }
                        }
                    });
                }
            }
        }
    };

    let download_file = move |_| {
        let content = converted_content.get_untracked();
        if content.is_empty() {
            return;
        }

        // Create a blob and download link
        spawn_local(async move {
            let result: Result<(), String> = (|| {
                let window = web_sys::window().ok_or("No window object available")?;
                let document = window
                    .document()
                    .ok_or("No document available")?;

                let array = js_sys::Array::new();
                array.push(&JsValue::from_str(&content));
                let blob = web_sys::Blob::new_with_str_sequence(&array)
                    .map_err(|_| "Failed to create blob".to_string())?;

                let url = web_sys::Url::create_object_url_with_blob(&blob)
                    .map_err(|_| "Failed to create download URL".to_string())?;

                let a = document
                    .create_element("a")
                    .map_err(|_| "Failed to create link element".to_string())?;

                a.set_attribute("href", &url)
                    .map_err(|_| "Failed to set href".to_string())?;
                a.set_attribute("download", "converted_hands.txt")
                    .map_err(|_| "Failed to set download attribute".to_string())?;

                let html_element = a
                    .dyn_into::<web_sys::HtmlElement>()
                    .map_err(|_| "Failed to convert to HtmlElement".to_string())?;

                html_element.click();

                web_sys::Url::revoke_object_url(&url)
                    .map_err(|_| "Failed to revoke URL".to_string())?;

                Ok(())
            })();

            if let Err(e) = result {
                set_upload_status.set(Some(format!("[ERR] Download failed: {}", e)));
            }
        });
    };

    let copy_to_clipboard = move |_| {
        let content = converted_content.get_untracked();
        if content.is_empty() {
            return;
        }

        spawn_local(async move {
            match web_sys::window() {
                Some(window) => {
                    let clipboard = window.navigator().clipboard();
                    let promise = clipboard.write_text(&content);
                    match wasm_bindgen_futures::JsFuture::from(promise).await {
                        Ok(_) => {
                            set_upload_status.set(Some("Copied to clipboard".to_string()));
                        }
                        Err(_) => {
                            set_upload_status
                                .set(Some("[ERR] Failed to write to clipboard".to_string()));
                        }
                    }
                }
                None => {
                    set_upload_status
                        .set(Some("[ERR] Window object not available".to_string()));
                }
            }
        });
    };

    view! {
        <div class="min-h-screen bg-gray-50 dark:bg-gray-900 py-12 px-4 sm:px-6 lg:px-8">
            <div class="max-w-4xl mx-auto">
                <div class="text-center mb-12">
                    <h1 class="text-4xl md:text-5xl font-bold text-gray-900 dark:text-white mb-4">
                        "OHH to PokerStars Converter"
                    </h1>
                    <p class="text-xl text-gray-600 dark:text-gray-300">
                        "Convert .ohh poker hand history files to PokerStars format for GTO Wizard"
                    </p>
                </div>

                <div class="bg-white dark:bg-gray-800 rounded-lg shadow-lg p-8 mb-8">
                    <h2 class="text-2xl font-bold text-gray-900 dark:text-white mb-6">
                        "Upload and Convert"
                    </h2>

                    <div
                        class="border-2 border-dashed rounded-lg p-12 text-center transition-all duration-200 cursor-pointer border-gray-300 dark:border-gray-600 hover:border-blue-400 dark:hover:border-blue-500"
                        on:dragover=on_drag_over
                        on:dragleave=on_drag_leave
                        on:drop=on_drop
                    >
                        <div class="space-y-4">
                            <div class="flex justify-center">
                                <svg class="w-16 h-16 text-gray-400 dark:text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"></path>
                                </svg>
                            </div>
                            <div>
                                <p class="text-lg text-gray-700 dark:text-gray-300 mb-2">
                                    "Drag and drop your file here"
                                </p>
                                <p class="text-sm text-gray-500 dark:text-gray-400 mb-4">
                                    "or"
                                </p>
                                <label class="inline-block bg-blue-600 hover:bg-blue-700 text-white font-semibold py-2 px-6 rounded-lg cursor-pointer transition-colors">
                                    "Browse Files"
                                    <input
                                        node_ref=file_input_ref
                                        type="file"
                                        accept=".ohh,.txt,.json"
                                        class="sr-only"
                                        on:change=on_file_input
                                    />
                                </label>
                            </div>
                            <p class="text-xs text-gray-500 dark:text-gray-400">
                                "Only .ohh, .txt, or .json files are accepted"
                            </p>
                        </div>
                    </div>

                    {move || selected_file_info.get().map(|file_info| {
                        view! {
                            <div class="mt-6 p-4 bg-gray-50 dark:bg-gray-700 rounded-lg">
                                <div class="flex items-center justify-between">
                                    <div class="flex items-center space-x-3">
                                        <svg class="w-8 h-8 text-blue-600 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"></path>
                                        </svg>
                                        <div>
                                            <p class="font-medium text-gray-900 dark:text-white">{file_info.name.clone()}</p>
                                            <p class="text-sm text-gray-500 dark:text-gray-400">
                                                {format!("{:.2} KB", file_info.size / 1024.0)}
                                            </p>
                                        </div>
                                    </div>
                                    <button
                                        class="bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 text-white font-semibold py-2 px-6 rounded-lg transition-colors"
                                        on:click=convert_file
                                        disabled=move || is_converting.get()
                                    >
                                        {move || if is_converting.get() { "Converting..." } else { "Convert" }}
                                    </button>
                                </div>
                            </div>
                        }
                    })}

                    {move || upload_status.get().map(|status| {
                        let is_error = status.starts_with("Error") || status.starts_with("[ERR]");
                        let is_success = status.starts_with("[OK]");
                        view! {
                            <div class=move || {
                                let base = "mt-4 p-4 rounded-lg";
                                if is_error {
                                    format!("{} bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400", base)
                                } else if is_success {
                                    format!("{} bg-green-50 dark:bg-green-900/20 text-green-700 dark:text-green-400", base)
                                } else {
                                    format!("{} bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-400", base)
                                }
                            }>
                                {status}
                            </div>
                        }
                    })}
                </div>

                {move || (!converted_content.get().is_empty()).then(|| {
                    view! {
                        <div class="bg-white dark:bg-gray-800 rounded-lg shadow-lg p-8">
                            <div class="flex justify-between items-center mb-4">
                                <h2 class="text-2xl font-bold text-gray-900 dark:text-white">
                                    "Converted Output"
                                </h2>
                                <div class="flex gap-2">
                                    <button class="bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 text-gray-900 dark:text-white font-semibold py-2 px-4 rounded-lg transition-colors" on:click=copy_to_clipboard>
                                        "Copy"
                                    </button>
                                    <button class="bg-blue-600 hover:bg-blue-700 text-white font-semibold py-2 px-4 rounded-lg transition-colors" on:click=download_file>
                                        "Download"
                                    </button>
                                </div>
                            </div>
                            <pre class="bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg p-4 overflow-auto max-h-96 text-sm text-gray-800 dark:text-gray-200">
                                { move || {
                                    let content = converted_content.get();
                                    if content.len() > 1000 {
                                        format!("{}...\n\n[{} total characters - download to see full output]",
                                                &content[..1000], content.len())
                                    } else {
                                        content
                                    }
                                }}
                            </pre>
                        </div>
                    }
                })}
            </div>
        </div>
    }
}

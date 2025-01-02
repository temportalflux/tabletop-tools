use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

pub mod components;

#[wasm_bindgen(module = "bootstrap")]
extern "C" {
	pub type Tooltip;
	#[wasm_bindgen(constructor)]
	pub fn new(element: JsValue, config: JsValue) -> Tooltip;
}

#[wasm_bindgen(module = "bootstrap")]
extern "C" {
	pub type Modal;
	#[wasm_bindgen(constructor)]
	pub fn new(element: JsValue, config: JsValue) -> Modal;
	#[wasm_bindgen(method)]
	pub fn show(this: &Modal, relatedTarget: JsValue);
	#[wasm_bindgen(method)]
	pub fn hide(this: &Modal);
}
impl Modal {
	pub fn from(value: &yew::NodeRef) -> Option<Self> {
		value.get().map(|node| Self::new(node.into(), JsValue::UNDEFINED))
	}
}

pub fn initialize_tooltips() {
	let doc = web_sys::window().unwrap().document().unwrap();
	if let Ok(list) = doc.query_selector_all("[data-bs-toggle=\"tooltip\"]") {
		for idx in 0..list.length() {
			if let Some(node) = list.get(idx) {
				Tooltip::new(node.into(), JsValue::from("{}".to_owned()));
			}
		}
	}
}

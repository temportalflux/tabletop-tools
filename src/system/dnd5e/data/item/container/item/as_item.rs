use super::ItemPath;
use crate::system::dnd5e::data::item::Item;

pub trait AsItem {
	fn from_item(item: Item) -> Self;

	fn set_id_path(&mut self, id: ItemPath);
	fn id_path(&self) -> Option<&ItemPath>;

	fn into_item(self) -> Item;
	fn as_item(&self) -> &Item;
	fn as_item_mut(&mut self) -> &mut Item;
}

impl AsItem for Item {
	fn from_item(item: Item) -> Self {
		item
	}

	fn set_id_path(&mut self, id: ItemPath) {
		if let Some(container) = &mut self.items {
			container.parent_item_id = id;
		}
	}

	fn id_path(&self) -> Option<&ItemPath> {
		None
	}

	fn into_item(self) -> Item {
		self
	}

	fn as_item(&self) -> &Item {
		self
	}

	fn as_item_mut(&mut self) -> &mut Item {
		self
	}
}

use crate::{
	kdl_ext::NodeContext,
	system::{
		dnd5e::data::{
			character::{Character, ObjectCacheProvider},
			currency::Wallet,
			item::{Item, Restriction},
			Indirect, Spell,
		},
		mutator::{self, ReferencePath},
		SourceId,
	},
};
use async_recursion::async_recursion;
use itertools::Itertools;
use kdlize::{ext::DocumentExt, AsKdl, FromKdl, NodeBuilder, OmitIfEmpty};
use std::{collections::HashMap, path::PathBuf};
use uuid::Uuid;

mod as_item;
pub use as_item::*;
mod equipable;
pub use equipable::*;
mod capacity;
pub use capacity::*;

pub type Inventory = ItemContainer<EquipableEntry>;

#[derive(Clone, PartialEq, Debug, Default)]
pub struct ItemPath(Vec<Uuid>);
impl<Id> From<Id> for ItemPath
where
	Id: AsRef<Uuid>,
{
	fn from(value: Id) -> Self {
		Self(vec![*value.as_ref()])
	}
}
impl ItemPath {
	pub fn len(&self) -> usize {
		self.0.len()
	}
	pub fn iter(&self) -> std::slice::Iter<'_, Uuid> {
		self.0.iter()
	}
	pub fn as_single(&self) -> Option<Uuid> {
		match self.0.len() == 1 {
			true => self.first(),
			false => None,
		}
	}
	pub fn first(&self) -> Option<Uuid> {
		self.0.get(0).map(|v| *v)
	}
	pub fn last(&self) -> Option<Uuid> {
		self.0.last().map(|v| *v)
	}
	pub fn container(&self) -> Option<ItemPath> {
		match self.0.len() {
			0 | 1 => None,
			n => Some(Self(self.0[0..(n - 1)].to_vec())),
		}
	}
	pub fn push(&mut self, id: impl AsRef<Uuid>) {
		self.0.push(*id.as_ref());
	}
	pub fn join(&self, id: impl AsRef<Uuid>) -> Self {
		let mut path = self.clone();
		path.push(id);
		path
	}
}
impl std::fmt::Display for ItemPath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0.iter().join("/"))
	}
}
impl std::str::FromStr for ItemPath {
	type Err = <Uuid as std::str::FromStr>::Err;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let mut path = Vec::new();
		for id_str in s.split("/") {
			path.push(Uuid::from_str(id_str)?);
		}
		Ok(Self(path))
	}
}

#[derive(Clone, PartialEq, Debug)]
pub struct ItemContainer<T> {
	parent_item_id: ItemPath,
	pub capacity: Capacity,
	pub restriction: Option<Restriction>,
	item_templates: HashMap<SourceId, usize>,
	items_by_id: HashMap<Uuid, T>,
	itemids_by_name: Vec<Uuid>,
	wallet: Wallet,
}

impl<T> Default for ItemContainer<T> {
	fn default() -> Self {
		Self {
			parent_item_id: Default::default(),
			capacity: Default::default(),
			restriction: Default::default(),
			item_templates: Default::default(),
			items_by_id: Default::default(),
			itemids_by_name: Default::default(),
			wallet: Default::default(),
		}
	}
}

impl<T> ItemContainer<T> {
	pub fn new() -> Self {
		Self {
			parent_item_id: Default::default(),
			capacity: Capacity::default(),
			restriction: None,
			item_templates: HashMap::new(),
			items_by_id: HashMap::new(),
			itemids_by_name: Vec::new(),
			wallet: Wallet::default(),
		}
	}

	pub fn is_empty(&self) -> bool {
		self.items_by_id.is_empty()
	}

	pub fn wallet(&self) -> &Wallet {
		&self.wallet
	}

	pub fn wallet_mut(&mut self) -> &mut Wallet {
		&mut self.wallet
	}

	pub fn iter_by_name(&self) -> impl Iterator<Item = (&Uuid, &T)> {
		self.itemids_by_name.iter().filter_map(|id| self.items_by_id.get(&id).map(|item| (id, item)))
	}
}

impl<T: AsItem> ItemContainer<T> {
	pub fn get_item(&self, id: &Uuid) -> Option<&Item> {
		self.items_by_id.get(id).map(|entry| entry.as_item())
	}

	pub fn get_mut(&mut self, id: &Uuid) -> Option<&mut Item> {
		self.items_by_id.get_mut(id).map(|entry| entry.as_item_mut())
	}

	pub fn get_mut_at_path<'c>(&'c mut self, id_path: &ItemPath) -> Option<&'c mut Item> {
		let mut iter = id_path.iter();
		let Some(first_id) = iter.next() else {
			return None;
		};
		let Some(mut item) = self.get_mut(first_id) else {
			return None;
		};
		for id in iter {
			let Some(container) = &mut item.items else {
				return None;
			};
			let Some(next_item) = container.get_mut(id) else {
				return None;
			};
			item = next_item;
		}
		Some(item)
	}

	fn push_entry(&mut self, mut entry: T) -> Uuid {
		let id = Uuid::new_v4();
		entry.set_id_path(self.parent_item_id.join(id));
		let search = self.itemids_by_name.binary_search_by(|id| {
			let Some(entry_item) = self.get_item(id) else {
				return std::cmp::Ordering::Less;
			};
			entry_item.name.cmp(&entry.as_item().name)
		});
		let idx = match search {
			// an item with the same name already exists at this index
			Ok(idx) => idx,
			// no item with the name exists, this is the index to insert to maintain sort-order
			Err(idx) => idx,
		};
		self.itemids_by_name.insert(idx, id.clone());
		self.items_by_id.insert(id.clone(), entry);
		id
	}

	pub fn push(&mut self, item: Item) -> Uuid {
		self.push_entry(T::from_item(item))
	}

	pub fn insert(&mut self, item: Item) -> Uuid {
		if item.can_stack() {
			for (id, entry) in &mut self.items_by_id {
				if entry.as_item().can_add_to_stack(&item) {
					entry.as_item_mut().add_to_stack(item);
					return id.clone();
				}
			}
		}
		self.push(item)
	}

	/// Attempts to insert the item into the specified item container.
	/// If no such item at the id exists OR that item is not an item container,
	/// the provided item is inserted into this inventory.
	pub fn insert_to(&mut self, item: Item, container_id: Option<&ItemPath>) -> ItemPath {
		if let Some(container_id) = container_id {
			if let Some(existing_item) = self.get_mut_at_path(container_id) {
				if let Some(container) = &mut existing_item.items {
					let id = container.insert(item);
					let mut full_path = container_id.clone();
					full_path.push(id);
					return full_path;
				}
			}
		}
		ItemPath::from(self.insert(item))
	}

	pub fn remove(&mut self, id: &Uuid) -> Option<Item> {
		if let Ok(idx) = self.itemids_by_name.binary_search(id) {
			self.itemids_by_name.remove(idx);
		}
		self.items_by_id.remove(id).map(|entry| entry.into_item())
	}

	pub fn remove_at_path(&mut self, id_path: &ItemPath) -> Option<Item> {
		let count = id_path.len();
		let mut iter = id_path.iter().enumerate().map(|(idx, id)| (id, idx == count - 1));
		let Some((first_id, single_entry)) = iter.next() else {
			return None;
		};
		if single_entry {
			return self.remove(first_id);
		}
		let Some(mut item) = self.get_mut(first_id) else {
			return None;
		};
		for (id, is_last) in iter {
			let Some(container) = &mut item.items else {
				return None;
			};
			if is_last {
				return container.remove(id);
			}
			let Some(next_item) = container.get_mut(id) else {
				return None;
			};
			item = next_item;
		}
		None
	}

	// Expands all Indirect items and spells contained within the container,
	// recursively visiting all items which contain other items or spells.
	#[async_recursion(?Send)]
	pub async fn resolve_indirection(&mut self, provider: &ObjectCacheProvider) -> anyhow::Result<()> {
		// Any item templates need to be resolved to their full items
		for (item_id, count) in self.item_templates.drain().collect::<Vec<_>>() {
			let Some(item) = provider.get_typed_entry::<Item>(item_id.unversioned(), None).await? else {
				log::error!(target: "inventory", "failed to find item {:?}", item_id.to_string());
				continue;
			};
			for item in item.create_stack(count) {
				self.insert(item);
			}
		}

		// Then we process all fully defined items (including those fetched in the loop above),
		// to expand any items or spells each item contains.
		for (_item_id, entry) in &mut self.items_by_id {
			if let Some(container) = &mut entry.as_item_mut().items {
				container.resolve_indirection(provider).await?;
			}
			if let Some(container) = &mut entry.as_item_mut().spells {
				for entry in &mut container.spells {
					if let Indirect::Id(spell_id) = &entry.spell {
						let Some(spell) = provider.get_typed_entry::<Spell>(spell_id.unversioned(), None).await? else {
							log::error!(target: "inventory", "failed to find spell {:?}", spell_id.to_string());
							continue;
						};
						entry.spell = Indirect::Custom(spell);
					}
				}
			}
		}

		Ok(())
	}
}

impl<T> FromKdl<NodeContext> for ItemContainer<T>
where
	T: AsItem + FromKdl<NodeContext>,
	anyhow::Error: From<T::Error>,
{
	type Error = anyhow::Error;
	fn from_kdl<'doc>(node: &mut crate::kdl_ext::NodeReader<'doc>) -> anyhow::Result<Self> {
		let wallet = node.query_opt_t::<Wallet>("scope() > wallet")?.unwrap_or_default();

		let capacity = match node.query_opt("scope() > capacity")? {
			Some(node) => {
				let count = node.get_i64_opt("count")?.map(|v| v as usize);
				let weight = node.get_f64_opt("weight")?;
				let volume = node.get_f64_opt("volume")?;
				Capacity { count, weight, volume }
			}
			None => Default::default(),
		};

		let restriction = match node.query_opt("scope() > restriction")? {
			Some(node) => {
				let tags = node.query_str_all("scope() > tag", 0)?;
				let tags = tags.into_iter().map(str::to_owned).collect::<Vec<_>>();
				Some(Restriction { tags, weapon: None })
			}
			None => None,
		};

		let mut inventory = Self {
			parent_item_id: ItemPath::default(),
			wallet,
			capacity,
			restriction,
			item_templates: HashMap::new(),
			items_by_id: HashMap::new(),
			itemids_by_name: Vec::new(),
		};

		for node in &mut node.query_all("scope() > item_id")? {
			let id = node.next_str_req_t::<SourceId>()?;
			let id = id.with_relative_basis(node.context().id(), false);
			let count = node.get_i64_opt("count")?.unwrap_or(1) as usize;
			match inventory.item_templates.get_mut(&id) {
				None => {
					inventory.item_templates.insert(id, count);
				}
				Some(amount) => {
					*amount += count;
				}
			}
		}
		for node in &mut node.query_all("scope() > item")? {
			let item = T::from_kdl(node)?;
			inventory.push_entry(item);
		}

		Ok(inventory)
	}
}

impl<T: AsKdl> AsKdl for ItemContainer<T> {
	fn as_kdl(&self) -> NodeBuilder {
		let mut node = NodeBuilder::default();

		node.child(("wallet", &self.wallet, OmitIfEmpty));

		node.child((
			{
				let mut node = NodeBuilder::default();
				if let Some(count) = &self.capacity.count {
					node.entry(("count", *count as i64));
				}
				if let Some(weight) = &self.capacity.weight {
					node.entry(("weight", *weight));
				}
				if let Some(volume) = &self.capacity.volume {
					node.entry(("volume", *volume));
				}
				node.build("capacity")
			},
			OmitIfEmpty,
		));

		if let Some(restriction) = &self.restriction {
			let mut restriction_node = NodeBuilder::default();
			restriction_node.children(("tag", restriction.tags.iter()));
			node.child((restriction_node.build("restriction"), OmitIfEmpty));
		}

		for (id, count) in &self.item_templates {
			let mut item_node = NodeBuilder::default();
			item_node += id.as_kdl();
			if *count > 1 {
				item_node.entry(("count", *count as i64));
			}
			node.child(item_node.build("item_id"));
		}
		for id in &self.itemids_by_name {
			let Some(entry) = self.items_by_id.get(id) else {
				continue;
			};
			node.child(entry.as_kdl().build("item"));
		}

		node
	}
}

impl Inventory {
	pub fn get_equip_status(&self, id: &Uuid) -> EquipStatus {
		let entry = self.items_by_id.get(id);
		let status = entry.map(|entry| entry.status);
		status.unwrap_or_default()
	}

	pub fn entries(&self) -> impl Iterator<Item = &EquipableEntry> {
		self.items_by_id.values()
	}

	pub fn set_equipped(&mut self, id: &Uuid, status: EquipStatus) {
		if let Some(entry) = self.items_by_id.get_mut(&id) {
			entry.status = status;
		}
	}
}

impl mutator::Group for Inventory {
	type Target = Character;

	fn set_data_path(&self, parent: &ReferencePath) {
		let path_to_self = parent.join("Inventory", None);
		for (_, entry) in self.iter_by_name() {
			entry.set_data_path(&path_to_self);
		}
	}

	fn apply_mutators(&self, stats: &mut Character, parent: &ReferencePath) {
		let path_to_self = parent.join("Inventory", None);
		for (_id, entry) in self.iter_by_name() {
			stats.apply_from(entry, &path_to_self);

			if entry.status == EquipStatus::Attuned {
				*stats.attunement_mut() += 1;
			}

			let path_to_item = parent.join(entry.id_as_path(), Some(PathBuf::from(&entry.item.name)));
			for tag in &entry.item.user_tags {
				stats.user_tags_mut().add_tag_usage(tag, &path_to_item);
			}
		}
	}
}

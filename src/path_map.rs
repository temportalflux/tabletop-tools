use std::{
	collections::BTreeMap,
	path::{Path, PathBuf},
};

#[derive(Clone, PartialEq, Debug)]
pub struct PathMap<T> {
	values: Vec<T>,
	children: BTreeMap<String, PathMap<T>>,
}

impl<T> Default for PathMap<T> {
	fn default() -> Self {
		Self { values: Vec::new(), children: BTreeMap::new() }
	}
}

impl<K, T, const N: usize> From<[(K, T); N]> for PathMap<T>
where
	K: AsRef<Path>,
{
	fn from(values: [(K, T); N]) -> Self {
		let mut map = Self::default();
		for (path, value) in values {
			map.insert(&path, value);
		}
		map
	}
}

impl<T> PathMap<T> {
	pub fn len(&self) -> usize {
		self.children.iter().fold(self.values.len(), |count, (_, child)| count + child.len())
	}

	fn get_mut_or_insert_at(&mut self, path: &Path) -> &mut PathMap<T> {
		let mut layer = self;
		for component in path.components() {
			let key = component.as_os_str().to_str().unwrap();
			if !layer.children.contains_key(key) {
				layer.children.insert(key.to_owned(), PathMap::default());
			}
			layer = layer.children.get_mut(key).unwrap();
		}
		layer
	}

	fn values(&mut self, key: impl AsRef<Path>) -> &mut Vec<T> {
		&mut self.get_mut_or_insert_at(key.as_ref()).values
	}

	pub fn push(&mut self, value: T) {
		self.values.push(value);
	}

	pub fn insert(&mut self, key: impl AsRef<Path>, value: T) {
		self.values(key).push(value);
	}

	pub fn set(&mut self, key: impl AsRef<Path>, value: T) {
		*self.values(key) = vec![value];
	}

	pub fn remove(&mut self, key: impl AsRef<Path>) -> Vec<T> {
		self.values(key).drain(..).collect()
	}

	pub fn as_vec(&self) -> Vec<(PathBuf, &T)> {
		let mut all_entries = Vec::with_capacity(self.len());
		self.push_as_vec_entries_to(&PathBuf::new(), &mut all_entries);
		all_entries
	}

	fn push_as_vec_entries_to<'c>(&'c self, path: &PathBuf, target: &mut Vec<(PathBuf, &'c T)>) {
		for value in &self.values {
			target.push((path.clone(), value));
		}
		for (key, child) in &self.children {
			child.push_as_vec_entries_to(&path.join(key), target);
		}
	}

	pub fn iter_values(&self) -> impl Iterator<Item = &T> {
		self.values.iter()
	}

	pub fn iter_values_mut(&mut self) -> impl Iterator<Item = &mut T> {
		self.values.iter_mut()
	}

	pub fn iter_children(&self) -> impl Iterator<Item = (&String, &PathMap<T>)> {
		self.children.iter()
	}

	pub fn get_all(&self, path: impl AsRef<Path>) -> Option<&PathMap<T>> {
		let mut map = self;
		for component in path.as_ref().components() {
			let key = component.as_os_str().to_str().unwrap();
			let Some(next_map) = map.children.get(key) else {
				return None;
			};
			map = next_map;
		}
		Some(&map)
	}

	pub fn get_all_mut(&mut self, path: impl AsRef<Path>) -> Option<&mut PathMap<T>> {
		let mut map = self;
		for component in path.as_ref().components() {
			let key = component.as_os_str().to_str().unwrap();
			let Some(next_map) = map.children.get_mut(key) else {
				return None;
			};
			map = next_map;
		}
		Some(map)
	}

	pub fn get(&self, path: impl AsRef<Path>) -> Option<&Vec<T>> {
		self.get_all(path).map(|map| &map.values)
	}

	pub fn get_mut(&mut self, path: impl AsRef<Path>) -> Option<&mut Vec<T>> {
		self.get_all_mut(path).map(|map| &mut map.values)
	}

	pub fn get_first(&self, path: impl AsRef<Path>) -> Option<&T> {
		self.get(path).map(|all| all.first()).flatten()
	}

	pub fn get_first_mut(&mut self, path: impl AsRef<Path>) -> Option<&mut T> {
		self.get_mut(path).map(|all| all.first_mut()).flatten()
	}
}

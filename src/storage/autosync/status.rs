use derivative::Derivative;
use std::{cell::RefCell, rc::Rc};
use yew::{hook, use_state_eq, AttrValue, UseStateHandle};

#[derive(Clone, Derivative)]
#[derivative(PartialEq)]
pub struct Status {
	#[derivative(PartialEq = "ignore")]
	rw_internal: Rc<RefCell<Vec<Stage>>>,
	r_external: UseStateHandle<Vec<Stage>>,
}
impl std::fmt::Debug for Status {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Status").field("State", &self.rw_internal).field("Display", &self.r_external).finish()
	}
}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct Stage {
	pub title: AttrValue,
	pub progress: Option<Progress>,
}

#[derive(Clone, PartialEq, Default, Debug)]
pub struct Progress {
	pub max: usize,
	pub progress: usize,
}

impl Status {
	fn mutate(&self, perform: impl FnOnce(&mut Vec<Stage>)) {
		let mut state = self.rw_internal.borrow_mut();
		perform(&mut *state);
		self.r_external.set(state.clone());
	}

	pub fn push_stage(&self, title: impl Into<AttrValue>, max_progress: Option<usize>) {
		self.mutate(move |state| {
			state.push(Stage { title: title.into(), progress: max_progress.map(|max| Progress { max, progress: 0 }) });
		});
	}

	pub fn pop_stage(&self) {
		self.mutate(move |state| {
			state.pop();
		});
	}

	pub fn set_progress_max(&self, max: usize) {
		self.mutate(move |state| {
			let Some(stage) = state.last_mut() else {
				log::error!(target: "autosync", "status has no stages");
				return;
			};
			let Some(progress) = &mut stage.progress else {
				log::error!(target: "autosync", "{stage:?} has no progress");
				return;
			};
			progress.max = max;
		});
	}

	pub fn increment_progress(&self) {
		self.mutate(move |state| {
			let Some(stage) = state.last_mut() else {
				log::error!(target: "autosync", "status has no stages");
				return;
			};
			let Some(progress) = &mut stage.progress else {
				log::error!(target: "autosync", "{stage:?} has no progress");
				return;
			};
			progress.progress = progress.max.min(progress.progress + 1);
		});
	}

	pub fn is_active(&self) -> bool {
		!self.r_external.is_empty()
	}

	pub fn stages(&self) -> &Vec<Stage> {
		&self.r_external
	}

	pub fn progress_max(&self) -> Option<usize> {
		let status = self.rw_internal.borrow();
		let stage = status.last()?;
		let progress = stage.progress.as_ref()?;
		Some(progress.max)
	}
}

#[hook]
pub fn use_status() -> Status {
	Status { rw_internal: Rc::new(RefCell::new(Default::default())), r_external: use_state_eq(|| Default::default()) }
}

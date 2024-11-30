use std::rc::Rc;
use yew::{hook, use_memo};

pub struct DataChannel<Request> {
	send_req: async_channel::Sender<Request>,
	recv_req: async_channel::Receiver<Request>,
}

impl<Request> DataChannel<Request> {
	pub fn try_send_req(&self, req: Request) {
		let _ = self.send_req.try_send(req);
	}

	pub fn receiver(&self) -> &async_channel::Receiver<Request> {
		&self.recv_req
	}
}

#[derive(Clone)]
pub struct Channel<Request>(pub Rc<DataChannel<Request>>);

impl<Request> PartialEq for Channel<Request> {
	fn eq(&self, other: &Self) -> bool {
		Rc::ptr_eq(&self.0, &other.0)
	}
}

impl<Request> std::ops::Deref for Channel<Request> {
	type Target = DataChannel<Request>;

	fn deref(&self) -> &Self::Target {
		&*self.0
	}
}

#[hook]
pub fn use_channel<Request: 'static>() -> Channel<Request> {
	Channel(use_memo((), |_| {
		let (send_req, recv_req) = async_channel::unbounded();
		DataChannel { send_req, recv_req }
	}))
}

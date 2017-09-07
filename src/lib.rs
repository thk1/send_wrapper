// Copyright 2017 Thomas Keh.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::ops::{Drop,Deref,DerefMut};
use std::marker::Send;
use std::thread;
use std::thread::ThreadId;

const DEREF_ERROR: &'static str = "Dropped SendWrapper<T> variable from a thread different to the one it has been created with.";
const DROP_ERROR: &'static str = "Dereferenced SendWrapper<T> variable from a thread different to the one it has been created with.";

pub struct SendWrapper<T> {
	data: *mut T,
	thread_id: ThreadId,
}

impl<T> SendWrapper<T> {

	/// Create a SendWrapper<T> wrapper around a value of type T.
	pub fn new(data: T) -> SendWrapper<T> {
		SendWrapper {
			data: Box::into_raw(Box::new(data)),
			thread_id: thread::current().id()
		}
	}

	pub fn valid(&self) -> bool {
		self.thread_id == thread::current().id()
	}

}

unsafe impl<T> Send for SendWrapper<T> { }

impl<T> Deref for SendWrapper<T> {
	type Target = T;

	/// Dereference to the contained value
	///
	/// # Panics
	/// Derefencing panics, if it is called from a different thread than the
	/// one the SendWrapper<T> instance has been created with.
	fn deref(&self) -> &T {
		if !self.valid() {
			panic!(DEREF_ERROR);
		}
		unsafe {
			&*self.data
		}
	}
}

impl<T> DerefMut for SendWrapper<T> {
	/// Dereference to the contained value
	///
	/// # Panics
	/// Derefencing panics, if it is called from a different thread than the
	/// one the SendWrapper<T> instance has been created with.
	fn deref_mut(&mut self) -> &mut T {
		if !self.valid() {
			panic!(DEREF_ERROR);
		}
		unsafe {
			&mut *self.data
		}
	}
}

impl<T> Drop for SendWrapper<T> {
	fn drop(&mut self) {
		if self.valid() {
			unsafe {
				let _dropper = Box::from_raw(self.data);
			}
		} else {
			if !std::thread::panicking() {
				// panic because of dropping from wrong thread
				// only do this while not unwinding (coud be caused by deref from wrong thread)
				panic!(DROP_ERROR);
			}
		}
	}
}

#[cfg(test)]
mod tests {

	use SendWrapper;
	use std::thread;
	use std::sync::mpsc::channel;
	use std::ops::Deref;

	#[test]
	fn test_deref() {
		let (sender, receiver) = channel();
		let w = SendWrapper::new(42);
		{
			let _x = w.deref();
		}
		let t = thread::spawn(move || {
			// move SendWrapper back to main thread, so it can be dropped from there
			sender.send(w).unwrap();
		});
		let w2 = receiver.recv().unwrap();
		{
			let _x = w2.deref();
		}
		assert!(t.join().is_ok());
	}

	#[test]
	fn test_deref_panic() {
		let w = SendWrapper::new(42);
		let t = thread::spawn(move || {
			let _x = w.deref();
		});
		let join_result = t.join();
		assert!(join_result.is_err());
	}

	#[test]
	fn test_drop_panic() {
		let w = SendWrapper::new(42);
		let t = thread::spawn(move || {
			let _x = w;
		});
		let join_result = t.join();
		assert!(join_result.is_err());
	}

	#[test]
	fn test_valid() {
		let w = SendWrapper::new(5usize);
		assert!(w.valid());
		thread::spawn(move || {
			assert!(!w.valid());
		});
	}

}

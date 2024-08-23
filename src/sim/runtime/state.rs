use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Weak,
};

use super::task::{Task, TaskId};

////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub(crate) struct State {
    task_queue: VecDeque<TaskId>,
    tasks: HashMap<TaskId, Task>,
}

impl State {
    pub fn take_task(&mut self) -> Option<Task> {
        // some tasks from queue may be already resolved,
        // (there can be duplicates in task queue
        // or tasks can be cancelled)
        while let Some(task_id) = self.task_queue.pop_front() {
            if let Some(task) = self.tasks.remove(&task_id) {
                return Some(task);
            }
        }
        None
    }

    pub fn add_task(&mut self, task: Task) {
        let id = task.id();
        let prev_task = self.tasks.insert(id, task);
        assert!(prev_task.is_none());
        self.push_task(id)
    }

    pub fn push_task(&mut self, task_id: TaskId) {
        self.task_queue.push_back(task_id)
    }
}

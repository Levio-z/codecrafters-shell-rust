use std::{
    sync::atomic::{AtomicUsize, Ordering},
    vec,
};

use super::prelude::*;
use crate::GLOBAL_JOB;
/// Jobs命令处理器
pub struct JobsCommand;

impl Builtin for JobsCommand {
    fn execute(
        &self,
        _params: Vec<String>,
        _context: &mut ExecutionContext,
    ) -> BuiltinCommandResult {
        let mut jobs = GLOBAL_JOB.lock().unwrap();
        let s = jobs.list_all_jobs();
        BuiltinCommandResult::new_with_stdout(s)
    }
}

#[derive(Debug)]
pub struct Job {
    pub id: usize,
    pub pid: u32,
    pub command: String,
    pub status: JobStatus,
    pub child: Option<std::process::Child>,
}
// 从1开始自增的原子变量
static JOB_ID: std::sync::atomic::AtomicUsize = AtomicUsize::new(1);
impl Job {
    pub fn run(command: String) -> usize {
        let job = Self {
            id: {
                let mut joblist = GLOBAL_JOB.lock().unwrap();
                joblist.next_id()
            },
            pid: 0,
            command,
            status: JobStatus::Running,
            child: None,
        };
        let mut jobs = GLOBAL_JOB.lock().unwrap();
        let id = job.id;
        jobs.jobs.push(job);
        id
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum JobStatus {
    Running,
    Done,
}

pub struct JobList {
    pub jobs: Vec<Job>,
    pub list: Vec<usize>,
}
impl JobList {
    pub fn new() -> Self {
        Self {
            jobs: Vec::new(),
            list: Vec::new(),
        }
    }
    pub fn next_id(&mut self) -> usize {
        if let Some(first) = self.list.first() {
            *first
        } else {
            JOB_ID.fetch_add(1, Ordering::SeqCst)
        }
    }

    fn insert(&mut self, value: usize) {
        let pos = self.list.binary_search(&value).unwrap_or_else(|e| e);

        self.list.insert(pos, value);
    }
    pub fn list_all_jobs(&mut self) -> String {
        // [1]+  Running
        if self.jobs.is_empty() {
            return String::new();
        }

        let s = self.list_jobs(false);
        self.sort_running_jobs();
        s
        // 运行 ——状态，总共填充 24 个字符。由于“奔跑”有 7 个字符，后面会跟 17
        // 个空格来填满整个栏
    }
    // 将下面提取为方法
    pub fn sort_running_jobs(&mut self) {
        self.jobs = core::mem::take(&mut self.jobs)
            .into_iter()
            .filter(|j| j.status == JobStatus::Running)
            .collect();
        self.jobs.sort_by_key(|a| a.id);
    }
    pub fn update_pid(&mut self, id: usize, pid: u32, child: std::process::Child) {
        let idx = id - 1;
        if let Some(job) = self.jobs.get_mut(idx) {
            job.pid = pid;
            job.child = Some(child);
        }
    }

    pub fn list_done_jobs(&mut self) -> String {
        let s = self.list_jobs(true);
        self.sort_running_jobs();
        s
    }
    /// list_jobs 列出所有作业
    pub fn list_jobs(&mut self, is_done: bool) -> String {
        let mut s = String::new();
        let len = self.jobs.len();
        let mut ids = vec![];
        for (idx, job) in self.jobs.iter_mut().enumerate() {
            if let Some(child) = job.child.as_mut()
                && let Ok(Some(_status)) = child.try_wait()
            {
                job.status = JobStatus::Done;
                ids.push(job.id);
            }
            let marker = if idx + 1 == len {
                '+'
            } else if idx + 2 == len {
                '-'
            } else {
                ' '
            };
            let suffix = if job.status == JobStatus::Running {
                "&"
            } else {
                ""
            };
            if job.status == JobStatus::Running && is_done {
                continue;
            }
            let status_string = format!("{:?}", job.status);
            s.push_str(&format!(
                "[{}]{}  {:<24}{}{}\n",
                job.id, marker, status_string, job.command, suffix
            ));
        }
        for id in ids {
            self.insert(id);
        }
        s
    }
}

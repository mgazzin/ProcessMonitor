#![allow(unused_must_use)]

extern crate sysinfo;
extern crate slack_hook;

use sysinfo::{ProcessExt, System, SystemExt,ProcessStatus};
use clap::{App, Arg};
use slack_hook::{PayloadBuilder, Slack};
use std::{
    thread,
    time::{Duration, Instant},
    env,
    fs,
    collections::HashMap,
};

#[macro_use]
extern crate log;

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
struct Process {
    process: Vec<PS>,
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
struct PS {
    name: String,
    count: u16,
}

fn check_processes(
    def: &str,
    sl_h_env: &str,
    sl_usr: &str,
    sl_ch: &str,
) {

    let p_res = serde_yaml::from_str(def);
    let p: Process = match p_res {
        Ok(proc) => proc,
        Err(_error) => { error!("can't parse monitor definition."); return }
    };

    let sl_res = Slack::new(sl_h_env);
    let sl = match sl_res {
        Ok(hook) => hook,
        Err(_error) => { error!("can't connect to Slack hook."); return }
    };

    info!("Checking the followig processes:");
    for ps in &p.process {
        info!("{}: {}", ps.name, ps.count)
    }

    info!("Getting processes information...");
    let t = System::new_all();
    let mut found_map = HashMap::new();
    for (pid, proc_) in t.get_processes() {
        debug!("{}:{} status={:?}", pid, proc_.name(), proc_.status());
        match proc_.status() {
            ProcessStatus::Run => {
                for ps in &p.process {
                    if ps.name == proc_.name() {
                        *found_map.entry(proc_.name()).or_insert(0) += 1;
                    }
                }
            }
        }
    }
    for (key, value) in &found_map {
        debug!("{}: {}", key, value);
        for ps in &p.process {
            debug!("{}: {}", ps.name, ps.count);
            if &ps.name == key {
                if value.clone() == ps.count {
                    info!("{}: {} matched", ps.name, ps.count);
                } else {
                    info!("{}: {} not matched [{}]", ps.name, ps.count, value);
                    let pay_load = PayloadBuilder::new()
                        .text(format!("{}: {} not matched [{}]", ps.name, ps.count, value))
                        .channel(sl_ch)
                        .username(sl_usr)
                        .icon_emoji(":chart_with_upwards_trend:")
                        .build()
                        .unwrap();

                    let res = sl.send(&pay_load);
                    match res {
                        Ok(()) => info!("ok"),
                        Err(_error) => { error!("can't send Slack message."); return }
                    }
                }
            }
        }
    }
}

fn main() {
    let matches = App::new("ProcessMonitor")
        .version("0.1.0")
        .author("Marco Gazzin <gazzin.marco@gmail.com>")
        .about("ProcessMonitor: Rust program that reads a configuration from yaml file and alerts when process are not available.")
        .arg(
            Arg::with_name("yaml")
                .short("y")
                .long("yaml")
                .takes_value(true)
                .help("Yaml configuration file path"),
        )
        .arg(
            Arg::with_name("interval")
                .short("i")
                .long("interval")
                .takes_value(false)
                .value_name("INTERVAL")
                .default_value("120")
                .help("Interval in seconds"),
        )
        .get_matches();

    env_logger::init();

    let slack_hook_env = env::var("SLACK_HOOK").expect("SLACK_HOOK not set");
    let slack_user = env::var("SLACK_USER").expect("SLACK_USER not set");
    let slack_channel = env::var("SLACK_CHANNEL").expect("SLACK_HOOK not set");

    let conffile = matches.value_of("yaml").unwrap_or("process_checker.yml");
    info!("Configuration file is: {}", conffile);
    let interval_str = matches.value_of("interval").unwrap_or("120");

    let ps_res = fs::read_to_string(conffile);
    let ps_definition = match ps_res {
        Ok(content) => content,
        Err(error) => {
            panic!("Can't deal with {}, just exit here", error);
        }
    };

    let str_split = ps_definition.split('\n');
    for s in str_split {
        info!("{}", s);
    }

    let interval = interval_str.parse::<u64>().unwrap();
    let interval = interval.clone();

    let scheduler = thread::spawn(move || {

        let wait_time = Duration::from_millis(1000*interval);

        loop {
            let start = Instant::now();
            info!("Scheduler starting at {:?}", start);

            let ps_definition = ps_definition.clone();
            let slack_hook_env = slack_hook_env.clone();
            let slack_user = slack_user.clone();
            let slack_channel = slack_channel.clone();

            let thread_check_processes = thread::spawn(move || {
                check_processes(&ps_definition, &slack_hook_env, &slack_user, &slack_channel)
            });

            thread_check_processes.join().expect("Thread panicked");

            let runtime = start.elapsed();

            if let Some(remaining) = wait_time.checked_sub(runtime) {
                info!(
                    "schedule slice has time left over; sleeping for {:?}",
                    remaining
                );
                thread::sleep(remaining);
            }
        }
    });

    scheduler.join().expect("Scheduler panicked");

}

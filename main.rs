use regex::Regex;
use serde::Deserialize;
use std::env;
use std::fs;
use subprocess::Exec;
use subprocess::Redirection::{Merge, Pipe};
use toml;

#[derive(Deserialize)]
struct Config {
    logo: Logo,
    toc: Toc,
}

#[derive(Deserialize)]
struct Logo {
    enable: bool,
    file: String,
    seconds: u32,
    x: String,
    y: String,
}

#[derive(Deserialize)]
struct Toc {
    y: String,
    font: String,
    fontsize: u32,
    fontcolor: String,
    backgroundcolor: String,
    progresscolor: String,
}

fn main() {
    if env::args().count() < 3 {
        println!(
            r"Usage: vtoc input output
    or vtoc input output seconds"
        );
        return;
    }
    // config
    let content = fs::read_to_string("config.toml").unwrap();
    let config = toml::from_str::<Config>(&content).unwrap();

    // probe video
    let probe_cmd = format!("ffprobe '{}'", env::args().nth(1).unwrap());
    let probe_out = Exec::shell(probe_cmd)
        .stdout(Pipe)
        .stderr(Merge)
        .capture()
        .unwrap()
        .stdout_str();

    let re = Regex::new(r" (\d+)x(\d+),").unwrap();
    let m = re.captures(&probe_out).unwrap();

    let w: u32 = m[1].parse().unwrap();

    let re = Regex::new(r"Duration: (\d\d):(\d\d):(\d\d)").unwrap();
    let m = re.captures(&probe_out).unwrap();
    let hour: u32 = m[1].parse().unwrap();
    let min: u32 = m[2].parse().unwrap();
    let sec: u32 = m[3].parse().unwrap();
    let total_sec = sec + hour * 60 * 60 + min * 60;

    let mut filters = Vec::new();
    // logo
    if config.logo.enable {
        let filter = format!(
            "overlay=x='if(lte(t,{}), {},NAN)':y={}",
            config.logo.seconds, config.logo.x, config.logo.y
        );
        filters.push(filter);
    }
    // background rect
    let filter = format!(
        "drawtext=text='{}':y={}:font='{}':fontsize={}:fontcolor=Blue@0.0:box=1:boxcolor={}",
        "中".repeat((w * 2 / config.toc.fontsize) as usize),
        config.toc.y,
        config.toc.font,
        config.toc.fontsize,
        config.toc.backgroundcolor
    );
    filters.push(filter);
    // progress bar
    let filter = format!(
        "drawtext=text='{}':x=t*w/{}-tw:y={}:font='{}':fontsize={}:fontcolor=Blue@0.0:box=1:boxcolor={}",
        "中".repeat((w*2/config.toc.fontsize) as usize), 
        total_sec,
        config.toc.y,
        config.toc.font,
        config.toc.fontsize,
        config.toc.progresscolor
    );
    filters.push(filter);
    // read toc
    let lines = fs::read_to_string("toc.txt").unwrap();
    let mut pre_time = total_sec;

    for line in lines.lines().rev() {
        // parse time and text
        let re = Regex::new(r"(\d\d):(\d\d):(\d\d)\s(.+)").unwrap();
        let m = re.captures(&line).unwrap();
        let hour: u32 = m[1].parse().unwrap();
        let min: u32 = m[2].parse().unwrap();
        let sec: u32 = m[3].parse().unwrap();
        let sec = sec + hour * 60 * 60 + min * 60;
        let text = &m[4];

        let x = sec * w / total_sec;
        let x2 = pre_time * w / total_sec;
        if sec != 0 {
            let filter = format!(
                "drawtext=text='|':x={}:y={}:font='{}':fontsize={}:fontcolor={}",
                x, config.toc.y, config.toc.font, config.toc.fontsize, config.toc.fontcolor
            );
            filters.push(filter);
        }

        let filter = format!(
            "drawtext=text='{}':x=({}+{}-tw)/2:y={}:font='{}':fontsize={}:fontcolor={}",
            text, x, x2, config.toc.y, config.toc.font, config.toc.fontsize, config.toc.fontcolor
        );
        filters.push(filter);
        pre_time = sec;
    }

    let mut cmd = format!(
        "ffmpeg -i '{}' -i '{}' -filter_complex \"{}\" -c:a copy '{}'",
        env::args().nth(1).unwrap(),
        config.logo.file,
        filters.join(", "),
        env::args().nth(2).unwrap()
    );

    if env::args().count() == 3 {
        Exec::shell(cmd).join().unwrap();
    } else {
        let extra = format!(" -t {}", env::args().nth(3).unwrap());
        cmd.insert_str(6, &extra);
        Exec::shell(cmd).join().unwrap();
    }
}

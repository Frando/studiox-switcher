use clap::Clap;
use faust_state::DspHandle;

mod config;
mod jack;
mod server;
// mod tui;
mod faust {
    include!(concat!(env!("OUT_DIR"), "/dsp.rs"));
}

#[derive(Clap)]
struct Opts {
    /// Path to config file
    #[clap(short, long)]
    pub config: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    let config = match opts.config {
        Some(config_path) => config::Config::load_from_path(config_path)?,
        None => config::Config::load_from_default_dirs()?,
    };

    let mut in_ports = vec![];
    in_ports.push(config.fallback_input.clone());
    in_ports.extend(config.inputs.clone());
    let port_spec = (in_ports, config.output.clone());

    // let config = config::Config::load()?;
    eprintln!("config {:#?}", config);
    let (dsp, state) = DspHandle::<faust::Switcher>::new();
    eprintln!("client name: {}", dsp.name());
    eprintln!("inputs: {}", dsp.num_inputs());
    eprintln!("outputs: {}", dsp.num_outputs());
    eprintln!("params: {:#?}", state.params());
    // eprintln!("meta: {:#?}", state.meta());

    // thread::spawn(move || {
    // });

    // Run the DSP as JACK client.
    let jack_handle = jack::start_dsp(dsp, Some(port_spec))?;

    ctrlc::set_handler(move || {
        jack_handle.stop();
        println!("Quitting...");
        std::process::exit(0);
    })?;

    server::run_server(state)?;
    Ok(())
    // wait
    // loop {
    //     thread::sleep(Duration::from_secs(10))
    // }
}

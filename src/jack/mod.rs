use faust_state::DspHandle;
use faust_types::FaustDsp;
use jack::AudioIn;
use jack::*;
use smallvec::SmallVec;
use std::fmt;
use std::sync::Mutex;
use std::{collections::HashMap, thread};

use crate::config::Port as PortConfig;

mod util;

use util::{ports_to_stereo, StereoPort};

pub type PortSpec = (Vec<PortConfig>, Option<PortConfig>);

pub fn start_dsp<T>(
    mut dsp: DspHandle<T>,
    port_spec: Option<PortSpec>,
) -> anyhow::Result<JackHandle>
where
    T: FaustDsp<T = f32> + 'static + Send,
{
    // Get number of inputs and ouputs
    let num_inputs = dsp.num_inputs();
    let num_outputs = dsp.num_outputs();

    let client_name = dsp.name().to_string();

    // Create JACK client
    let (client, in_ports, mut out_ports) =
        create_jack_client(dsp.name(), num_inputs as usize, num_outputs as usize)?;

    let in_port_names = in_ports
        .iter()
        .map(|p| p.name().unwrap().to_string())
        .collect::<Vec<String>>();
    let out_port_names = out_ports
        .iter()
        .map(|p| p.name().unwrap().to_string())
        .collect::<Vec<String>>();

    // Init DSP with a given sample rate
    dsp.init(client.sample_rate() as i32);

    let (conn_handler, mut command_rx) = if let Some(port_spec) = port_spec {
        ConnectionMap::from_ports_and_spec(&in_port_names[..], &out_port_names[..], &port_spec)?
    } else {
        ConnectionMap::new()
    };

    // Create JACK process closure that runs for each buffer
    let process_callback = {
        move |client: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
            while let Ok(command) = command_rx.pop() {
                handle_command(&client, &command);
            }

            // TODO: Make sure that this doesn't allocate.
            let mut inputs = SmallVec::<[&[f32]; 64]>::with_capacity(num_inputs as usize);
            let mut outputs = SmallVec::<[&mut [f32]; 64]>::with_capacity(num_outputs as usize);
            let len = ps.n_frames();
            for port in in_ports.iter() {
                inputs.push(port.as_slice(ps));
            }
            for port in out_ports.iter_mut() {
                outputs.push(port.as_mut_slice(ps));
            }

            // Call the update_and_compute handler on the Faust DSP. This first processes param changes
            // from the State handler and then computes the outputs from the inputs and params.
            dsp.update_and_compute(len as i32, &inputs[..], &mut outputs[..]);
            jack::Control::Continue
        }
    };

    // Init JACK process handler.
    let process = jack::ClosureProcessHandler::new(process_callback);

    // Activate the client, which starts the processing.
    let active_client = client.activate_async(conn_handler, process).unwrap();
    log::info!("registered JACK client: {}", client_name);
    log::info!("inputs {} outputs {}", num_inputs, num_outputs);

    let (cancel_tx, cancel_rx) = std::sync::mpsc::channel::<()>();

    // Wait for a cancel signal and then close the JACK client.
    thread::spawn(move || {
        let _ = cancel_rx.recv();
        active_client
            .deactivate()
            .expect("Failed to cleanly deactivate JACK client");
    });

    let handle = JackHandle { cancel_tx };
    Ok(handle)
}

fn handle_command(client: &Client, command: &Command) {
    let _res = match command {
        Command::Connect(pair) => pair.connect(&client),
        Command::Disconnect(pair) => pair.disconnect(&client),
    };
}

pub struct JackHandle {
    cancel_tx: std::sync::mpsc::Sender<()>,
}

impl JackHandle {
    pub fn stop(&self) {
        let _ = self.cancel_tx.send(());
    }
}

#[derive(Debug)]
pub struct PortPair {
    from: String,
    to: String,
}

impl PortPair {
    pub fn new<S1: ToString, S2: ToString>(from: S1, to: S2) -> Self {
        Self {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    pub fn connect(&self, client: &Client) -> Result<(), jack::Error> {
        client.connect_ports_by_name(&self.from, &self.to)
    }

    pub fn disconnect(&self, client: &Client) -> Result<(), jack::Error> {
        client.disconnect_ports_by_name(&self.from, &self.to)
    }
}

#[derive(Debug)]
pub enum Command {
    Connect(PortPair),
    Disconnect(PortPair),
}

fn create_jack_client(
    name: &str,
    num_inputs: usize,
    num_outputs: usize,
) -> anyhow::Result<(jack::Client, Vec<Port<AudioIn>>, Vec<Port<AudioOut>>)> {
    let (client, _status) = jack::Client::new(name, jack::ClientOptions::NO_START_SERVER).unwrap();
    let mut in_ports: Vec<Port<AudioIn>> = Vec::new();
    let mut out_ports: Vec<Port<AudioOut>> = Vec::new();

    for i in 0..num_inputs {
        let port = client
            .register_port(&format!("in{}", i), jack::AudioIn::default())
            .unwrap();
        in_ports.push(port);
    }
    for i in 0..num_outputs {
        let port = client
            .register_port(&format!("out{}", i), jack::AudioOut::default())
            .unwrap();
        out_ports.push(port);
    }

    Ok((client, in_ports, out_ports))
}

pub struct ConnectionMap {
    conns: HashMap<String, String>,
    command_tx: Mutex<rtrb::Producer<Command>>,
}

impl fmt::Debug for ConnectionMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConnectionMap")
            .field("conns", &self.conns)
            .finish()
    }
}

impl ConnectionMap {
    pub fn new() -> (Self, rtrb::Consumer<Command>) {
        let (command_tx, command_rx) = rtrb::RingBuffer::new(512).split();
        (Self::with_command_tx(command_tx), command_rx)
    }

    pub fn with_command_tx(command_tx: rtrb::Producer<Command>) -> Self {
        Self {
            command_tx: Mutex::new(command_tx),
            conns: HashMap::new(),
        }
    }
    pub fn _connect_all(&self, client: &Client, strict: bool) -> Result<(), jack::Error> {
        for (from, to) in self.conns.iter() {
            // let res = from.connect_to(&client, &to);
            let res = client.connect_ports_by_name(&from, &to);
            if strict {
                res?;
            }
        }
        Ok(())
    }

    pub fn _connections(&self) -> impl Iterator<Item = (&String, &String)> {
        self.conns.iter()
    }

    pub fn insert_stereo(&mut self, from_port: &StereoPort, to_port: &StereoPort) {
        self.conns
            .insert(from_port[0].to_string(), to_port[0].to_string());
        self.conns
            .insert(from_port[1].to_string(), to_port[1].to_string());
    }

    pub fn from_ports_and_spec(
        in_ports: &[String],
        out_ports: &[String],
        spec: &PortSpec,
    ) -> anyhow::Result<(Self, rtrb::Consumer<Command>)> {
        let (mut map, command_rx) = Self::new();
        let in_ports = ports_to_stereo(in_ports)?;
        let out_ports = ports_to_stereo(out_ports)?;

        let (in_targets, out_target) = spec;
        let in_targets: Vec<StereoPort> = in_targets
            .iter()
            .filter_map(|s| s.ports.as_ref().map(|s| StereoPort::new(&s[0], &s[1])))
            .collect();

        let out_target: Option<StereoPort> = out_target
            .as_ref()
            .and_then(|s| s.ports.as_ref())
            .map(StereoPort::from);

        let input_map_len = in_ports.len().min(in_targets.len());
        for i in 0..input_map_len {
            // map.insert_stereo(&in_ports[i], &in_targets[i])
            map.insert_stereo(&in_targets[i], &in_ports[i])
        }
        if let Some(out_target) = out_target {
            map.insert_stereo(&out_ports[0], &out_target)
            // map.insert_stereo(&out_target, &out_ports[0])
        }
        Ok((map, command_rx))
    }
}

impl jack::NotificationHandler for ConnectionMap {
    fn port_registration(&mut self, client: &Client, port_id: u32, is_registered: bool) {
        if !is_registered {
            return;
        }
        let port = client.port_by_id(port_id).unwrap();
        let mut command_tx = self.command_tx.lock().unwrap();
        for (from, to) in self.conns.iter() {
            if from == &port.name().unwrap() {
                let _ = command_tx.push(Command::Connect(PortPair::new(from, to)));
            }
            if to == &port.name().unwrap() {
                let _ = command_tx.push(Command::Connect(PortPair::new(from, to)));
            }
        }
    }
}

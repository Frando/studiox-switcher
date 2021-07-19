use faust_state::DspHandle;
use faust_types::FaustDsp;
use jack::AudioIn;
use jack::*;
use smallvec::SmallVec;

use std::thread;

use crate::config::Port as PortConfig;

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

    // Create JACK process closure that runs for each buffer
    let process_callback = {
        move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
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
    // let active_client = jack::AsyncClient::new(client, (), process).unwrap();
    let active_client = client.activate_async((), process).unwrap();

    if let Some(port_spec) = port_spec {
        connect_ports(
            active_client.as_client(),
            &in_port_names[..],
            &out_port_names[..],
            &port_spec,
        )?;
    }

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

pub struct JackHandle {
    cancel_tx: std::sync::mpsc::Sender<()>,
}

impl JackHandle {
    pub fn stop(&self) {
        let _ = self.cancel_tx.send(());
    }
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

pub fn connect_ports(
    client: &jack::Client,
    in_ports: &[String],
    out_ports: &[String],
    spec: &PortSpec,
) -> anyhow::Result<()> {
    let (in_targets, out_target) = spec;
    let input_len = (in_ports.len().min(in_targets.len() * 2)) / 2;
    for i in 0..input_len {
        let n = i * 2;
        let in_port_l = in_ports.get(n).unwrap();
        let in_port_r = in_ports.get(n + 1).unwrap();

        let in_target_l = &in_targets.get(i).as_ref().unwrap().ports[0];
        let in_target_r = &in_targets.get(i).as_ref().unwrap().ports[1];

        client.connect_ports_by_name(&in_target_l, &in_port_l)?;
        client.connect_ports_by_name(&in_target_r, &in_port_r)?;
        eprintln!("connected: {} -> {}", in_port_l, in_target_l);
        eprintln!("connected: {} -> {}", in_port_r, in_target_r);
    }
    if let Some(out_target) = out_target {
        let out_target_l = &out_target.ports[0];
        let out_target_r = &out_target.ports[1];
        let out_port_l = out_ports.get(0).unwrap();
        let out_port_r = out_ports.get(1).unwrap();
        client.connect_ports_by_name(&out_port_l, out_target_l)?;
        client.connect_ports_by_name(&out_port_r, out_target_r)?;
    }
    Ok(())
}

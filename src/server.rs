use async_osc::{prelude::*, OscPacket, OscSocket, OscType};
use async_std::stream::StreamExt;
use faust_state::StateHandle;

pub fn run_server(state: StateHandle) -> anyhow::Result<()> {
    async_std::task::block_on(run_server_async(state))
}

async fn run_server_async(mut dsp_state: StateHandle) -> anyhow::Result<()> {
    let addr = "localhost:7000";
    let mut socket = OscSocket::bind(addr).await?;
    log::info!("OSC server listening on {}", addr);
    while let Some(packet) = socket.next().await {
        let (packet, peer_addr) = packet?;
        log::debug!("Receive from {}: {:?}", peer_addr, packet);
        match packet {
            OscPacket::Bundle(_) => {}
            OscPacket::Message(message) => {
                // eprintln!("{:#?}", message);

                match &message.as_tuple() {
                    ("/state", &[]) => {
                        dsp_state.update();
                        // let mut params: Vec<_> = dsp_state.params().iter().collect();
                        // eprintln!("PRE {:?}", params);
                        // params.sort_by(|a, b| a.1.path().cmp(&b.1.path()));
                        // eprintln!("POST {:?}", params);
                        // for (_, param) in params {
                        for (path, value) in dsp_state.params_by_path() {
                            eprintln!("param {}: {:?}", path, value);
                        }
                        // for i in 0..3 {
                        //     eprintln!(
                        //         "input {}: {:?}",
                        //         i,
                        //         dsp_state.get_by_path(&format!("{}", i))
                        //     );
                        // }
                    }
                    ("/switcher", &[OscType::Int(ch), OscType::Int(active)]) => {
                        eprintln!("Set channel {} to {}", ch, active);
                        dsp_state.set_by_path(&format!("active/{}", ch), active as f32);
                        dsp_state.update();
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

use async_osc::{prelude::*, OscPacket, OscSocket, OscType};
use async_std::stream::StreamExt;
use faust_state::StateHandle;

pub fn run_server(state: StateHandle) -> anyhow::Result<()> {
    async_std::task::block_on(run_server_async(state))
}

async fn run_server_async(mut dsp_state: StateHandle) -> anyhow::Result<()> {
    let mut socket = OscSocket::bind("localhost:7000").await?;
    while let Some(packet) = socket.next().await {
        let (packet, peer_addr) = packet?;
        eprintln!("Receive from {}: {:?}", peer_addr, packet);
        match packet {
            OscPacket::Bundle(_) => {}
            OscPacket::Message(message) => {
                eprintln!("{:#?}", message);

                match &message.as_tuple() {
                    ("/state", &[]) => {
                        dsp_state.update();
                        for i in 0..3 {
                            eprintln!(
                                "input {}: {:?}",
                                i,
                                dsp_state.get_by_path(&format!("{}", i))
                            );
                        }
                    }
                    ("/switcher", &[OscType::Int(ch), OscType::Int(active)]) => {
                        // let state = match state {
                        //     0 => false,
                        //     _ => true
                        // };
                        eprintln!("Set channel {} to {}", ch, active);
                        dsp_state.set_by_path(&format!("{}", ch), active as f32);
                        dsp_state.update();
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

pub struct StereoPort(pub [String; 2]);

impl StereoPort {
    pub fn new<S1: AsRef<str>, S2: AsRef<str>>(port1: S1, port2: S2) -> Self {
        Self([port1.as_ref().to_string(), port2.as_ref().to_string()])
    }

    pub fn names(&self) -> &[String; 2] {
        &self.0
    }

    pub fn connect_to(&self, client: &jack::Client, other: &StereoPort) -> Result<(), jack::Error> {
        client.connect_ports_by_name(&self[0], &other[0])?;
        client.connect_ports_by_name(&self[1], &other[1])?;
        Ok(())
    }
}

impl From<[String; 2]> for StereoPort {
    fn from(ports: [String; 2]) -> Self {
        Self::new(&ports[0], &ports[1])
    }
}

impl<'a> From<&'a [String]> for StereoPort {
    fn from(ports: &'a [String]) -> Self {
        Self::new(&ports[0], &ports[1])
    }
}

impl<'a> From<&'a [String; 2]> for StereoPort {
    fn from(ports: &'a [String; 2]) -> Self {
        Self::new(&ports[0], &ports[1])
    }
}

impl<'a> std::ops::Index<usize> for &'a StereoPort {
    type Output = String;

    fn index(&self, idx: usize) -> &Self::Output {
        &self.0[idx]
    }
}

pub fn ports_to_stereo(ports: &[String]) -> anyhow::Result<Vec<StereoPort>> {
    if ports.len() % 2 != 0 {
        anyhow::bail!("Ports count must be even");
    }
    Ok(ports.chunks(2).map(StereoPort::from).collect())
}

use alloc::vec::Vec;
use core::convert::TryInto;

pub const WPA_NONCE_LEN: usize = 32;
pub const WPA_KEY_RSC_LEN: usize = 8;
pub const WPA_REPLAY_COUNTER_LEN: usize = 8;
pub const WPA_MIC_LEN: usize = 16;
pub const WPA_PMK_LEN: usize = 32;
pub const WPA_PTK_LEN: usize = 64;
pub const WPA_GTK_MAX_LEN: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WpaVersion {
    Wpa,
    Wpa2,
    Wpa3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WpaKeyMgmt {
    None,
    Wpa2Psk,
    Wpa2Enterprise,
    Wpa3Sae,
    Wpa3Enterprise,
    Owe,
    Fils,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WpaCipher {
    None,
    Wep40,
    Wep104,
    Tkip,
    Ccmp,
    Ccmp256,
    Gcmp,
    Gcmp256,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EapolKeyType {
    Rc4 = 1,
    Rsn = 2,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyInfo {
    KeyType = 0x0008,
    Install = 0x0040,
    KeyAck = 0x0080,
    KeyMic = 0x0100,
    Secure = 0x0200,
    Error = 0x0400,
    Request = 0x0800,
    EncKeyData = 0x1000,
}

#[repr(C, packed)]
pub struct EapolHeader {
    pub version: u8,
    pub packet_type: u8,
    pub packet_body_len: u16,
}

impl EapolHeader {
    pub fn new(version: u8, packet_type: u8, body_len: u16) -> Self {
        Self {
            version,
            packet_type,
            packet_body_len: body_len.to_be(),
        }
    }
}

#[repr(C, packed)]
pub struct EapolKey {
    pub descriptor_type: u8,
    pub key_info: u16,
    pub key_length: u16,
    pub replay_counter: [u8; WPA_REPLAY_COUNTER_LEN],
    pub nonce: [u8; WPA_NONCE_LEN],
    pub key_iv: [u8; 16],
    pub key_rsc: [u8; WPA_KEY_RSC_LEN],
    pub key_id: [u8; 8],
    pub key_mic: [u8; WPA_MIC_LEN],
    pub key_data_length: u16,
}

impl EapolKey {
    pub fn new(descriptor_type: EapolKeyType) -> Self {
        Self {
            descriptor_type: descriptor_type as u8,
            key_info: 0,
            key_length: 0,
            replay_counter: [0; WPA_REPLAY_COUNTER_LEN],
            nonce: [0; WPA_NONCE_LEN],
            key_iv: [0; 16],
            key_rsc: [0; WPA_KEY_RSC_LEN],
            key_id: [0; 8],
            key_mic: [0; WPA_MIC_LEN],
            key_data_length: 0,
        }
    }

    pub fn set_key_info(&mut self, flags: u16) {
        self.key_info = flags.to_be();
    }

    pub fn get_key_info(&self) -> u16 {
        u16::from_be(self.key_info)
    }

    pub fn set_key_ack(&mut self, ack: bool) {
        let mut info = self.get_key_info();
        if ack {
            info |= KeyInfo::KeyAck as u16;
        } else {
            info &= !(KeyInfo::KeyAck as u16);
        }
        self.set_key_info(info);
    }

    pub fn set_key_mic(&mut self, mic: bool) {
        let mut info = self.get_key_info();
        if mic {
            info |= KeyInfo::KeyMic as u16;
        } else {
            info &= !(KeyInfo::KeyMic as u16);
        }
        self.set_key_info(info);
    }

    pub fn set_install(&mut self, install: bool) {
        let mut info = self.get_key_info();
        if install {
            info |= KeyInfo::Install as u16;
        } else {
            info &= !(KeyInfo::Install as u16);
        }
        self.set_key_info(info);
    }

    pub fn set_secure(&mut self, secure: bool) {
        let mut info = self.get_key_info();
        if secure {
            info |= KeyInfo::Secure as u16;
        } else {
            info &= !(KeyInfo::Secure as u16);
        }
        self.set_key_info(info);
    }
}

pub struct Pmk {
    pub key: [u8; WPA_PMK_LEN],
}

impl Pmk {
    pub fn from_passphrase(passphrase: &str, ssid: &[u8]) -> Self {
        let mut pmk = [0u8; WPA_PMK_LEN];
        pbkdf2_sha256(passphrase.as_bytes(), ssid, 4096, &mut pmk);
        Self { key: pmk }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ()> {
        if bytes.len() != WPA_PMK_LEN {
            return Err(());
        }
        let mut key = [0u8; WPA_PMK_LEN];
        key.copy_from_slice(bytes);
        Ok(Self { key })
    }
}

pub struct Ptk {
    pub kck: Vec<u8>,
    pub kek: Vec<u8>,
    pub tk: Vec<u8>,
}

impl Ptk {
    pub fn derive(
        pmk: &Pmk,
        aa: &[u8; 6],
        spa: &[u8; 6],
        anonce: &[u8; WPA_NONCE_LEN],
        snonce: &[u8; WPA_NONCE_LEN],
        cipher: WpaCipher,
    ) -> Self {
        let mut data = Vec::with_capacity(76);
        
        if aa < spa {
            data.extend_from_slice(aa);
            data.extend_from_slice(spa);
        } else {
            data.extend_from_slice(spa);
            data.extend_from_slice(aa);
        }
        
        if anonce < snonce {
            data.extend_from_slice(anonce);
            data.extend_from_slice(snonce);
        } else {
            data.extend_from_slice(snonce);
            data.extend_from_slice(anonce);
        }

        let ptk_len = match cipher {
            WpaCipher::Tkip => 64,
            WpaCipher::Ccmp => 48,
            WpaCipher::Ccmp256 => 64,
            WpaCipher::Gcmp | WpaCipher::Gcmp256 => 64,
            _ => 48,
        };

        let mut ptk = vec![0u8; ptk_len];
        prf_sha256(&pmk.key, b"Pairwise key expansion", &data, &mut ptk);

        let (kck_len, kek_len) = match cipher {
            WpaCipher::Ccmp256 | WpaCipher::Gcmp256 => (24, 32),
            _ => (16, 16),
        };

        Self {
            kck: ptk[0..kck_len].to_vec(),
            kek: ptk[kck_len..kck_len + kek_len].to_vec(),
            tk: ptk[kck_len + kek_len..].to_vec(),
        }
    }
}

pub struct Gtk {
    pub key: Vec<u8>,
    pub key_id: u8,
    pub tx_seq: u64,
    pub rx_seq: u64,
}

impl Gtk {
    pub fn new(key_len: usize, key_id: u8) -> Self {
        let mut key = vec![0u8; key_len];
        generate_random_bytes(&mut key);
        
        Self {
            key,
            key_id,
            tx_seq: 0,
            rx_seq: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HandshakeState {
    Idle,
    Message1Sent,
    Message2Received,
    Message3Sent,
    Message4Received,
    Completed,
    Failed,
}

pub struct FourWayHandshake {
    pub state: HandshakeState,
    pub pmk: Pmk,
    pub anonce: [u8; WPA_NONCE_LEN],
    pub snonce: [u8; WPA_NONCE_LEN],
    pub ptk: Option<Ptk>,
    pub gtk: Option<Gtk>,
    pub replay_counter: u64,
    pub retry_count: u32,
}

impl FourWayHandshake {
    pub fn new(pmk: Pmk) -> Self {
        let mut anonce = [0u8; WPA_NONCE_LEN];
        generate_random_bytes(&mut anonce);
        
        Self {
            state: HandshakeState::Idle,
            pmk,
            anonce,
            snonce: [0; WPA_NONCE_LEN],
            ptk: None,
            gtk: None,
            replay_counter: 0,
            retry_count: 0,
        }
    }

    pub fn create_message1(&mut self) -> Vec<u8> {
        let mut frame = Vec::new();
        
        let eapol_hdr = EapolHeader::new(2, 3, 95);
        frame.extend_from_slice(unsafe {
            core::slice::from_raw_parts(
                &eapol_hdr as *const _ as *const u8,
                core::mem::size_of::<EapolHeader>(),
            )
        });
        
        let mut key = EapolKey::new(EapolKeyType::Rsn);
        key.set_key_ack(true);
        key.nonce = self.anonce;
        key.replay_counter = self.replay_counter.to_be_bytes();
        
        frame.extend_from_slice(unsafe {
            core::slice::from_raw_parts(
                &key as *const _ as *const u8,
                core::mem::size_of::<EapolKey>(),
            )
        });
        
        self.state = HandshakeState::Message1Sent;
        self.replay_counter += 1;
        
        frame
    }

    pub fn process_message2(
        &mut self,
        frame: &[u8],
        aa: &[u8; 6],
        spa: &[u8; 6],
    ) -> Result<(), ()> {
        if self.state != HandshakeState::Message1Sent {
            return Err(());
        }

        if frame.len() < core::mem::size_of::<EapolHeader>() + core::mem::size_of::<EapolKey>() {
            return Err(());
        }

        let key_offset = core::mem::size_of::<EapolHeader>();
        let key = unsafe {
            &*(frame[key_offset..].as_ptr() as *const EapolKey)
        };

        self.snonce = key.nonce;
        
        self.ptk = Some(Ptk::derive(
            &self.pmk,
            aa,
            spa,
            &self.anonce,
            &self.snonce,
            WpaCipher::Ccmp,
        ));
        
        if !self.verify_mic(frame, &self.ptk.as_ref().unwrap().kck) {
            self.state = HandshakeState::Failed;
            return Err(());
        }
        
        self.state = HandshakeState::Message2Received;
        Ok(())
    }

    pub fn create_message3(&mut self) -> Vec<u8> {
        if self.gtk.is_none() {
            self.gtk = Some(Gtk::new(16, 1));
        }
        
        let mut frame = Vec::new();
        
        let gtk_data = self.create_gtk_kde();
        let eapol_hdr = EapolHeader::new(2, 3, 95 + gtk_data.len() as u16);
        frame.extend_from_slice(unsafe {
            core::slice::from_raw_parts(
                &eapol_hdr as *const _ as *const u8,
                core::mem::size_of::<EapolHeader>(),
            )
        });
        
        let mut key = EapolKey::new(EapolKeyType::Rsn);
        key.set_key_ack(true);
        key.set_key_mic(true);
        key.set_install(true);
        key.set_secure(false);
        key.nonce = self.anonce;
        key.replay_counter = self.replay_counter.to_be_bytes();
        key.key_data_length = (gtk_data.len() as u16).to_be();
        
        frame.extend_from_slice(unsafe {
            core::slice::from_raw_parts(
                &key as *const _ as *const u8,
                core::mem::size_of::<EapolKey>(),
            )
        });
        
        frame.extend_from_slice(&gtk_data);
        
        if let Some(ptk) = &self.ptk {
            self.calculate_mic(&mut frame, &ptk.kck);
        }
        
        self.state = HandshakeState::Message3Sent;
        self.replay_counter += 1;
        
        frame
    }

    pub fn process_message4(&mut self, frame: &[u8]) -> Result<(), ()> {
        if self.state != HandshakeState::Message3Sent {
            return Err(());
        }

        if frame.len() < core::mem::size_of::<EapolHeader>() + core::mem::size_of::<EapolKey>() {
            return Err(());
        }

        if let Some(ptk) = &self.ptk {
            if !self.verify_mic(frame, &ptk.kck) {
                self.state = HandshakeState::Failed;
                return Err(());
            }
        }
        
        self.state = HandshakeState::Completed;
        Ok(())
    }

    fn create_gtk_kde(&self) -> Vec<u8> {
        let mut kde = Vec::new();
        
        kde.push(0xDD);
        
        if let Some(gtk) = &self.gtk {
            let kde_len = 6 + gtk.key.len();
            kde.push(kde_len as u8);
            
            kde.extend_from_slice(&[0x00, 0x0F, 0xAC]);
            kde.push(0x01);
            
            kde.push(gtk.key_id & 0x03);
            kde.push(0);
            
            kde.extend_from_slice(&gtk.key);
        }
        
        kde
    }

    fn calculate_mic(&self, frame: &mut [u8], kck: &[u8]) {
        let key_offset = core::mem::size_of::<EapolHeader>();
        let mic_offset = key_offset + 77;
        
        for i in 0..WPA_MIC_LEN {
            frame[mic_offset + i] = 0;
        }
        
        let mic = hmac_sha256(kck, frame);
        
        frame[mic_offset..mic_offset + WPA_MIC_LEN]
            .copy_from_slice(&mic[..WPA_MIC_LEN]);
    }

    fn verify_mic(&self, frame: &[u8], kck: &[u8]) -> bool {
        let mut frame_copy = frame.to_vec();
        let key_offset = core::mem::size_of::<EapolHeader>();
        let mic_offset = key_offset + 77;
        
        let original_mic = &frame[mic_offset..mic_offset + WPA_MIC_LEN];
        
        for i in 0..WPA_MIC_LEN {
            frame_copy[mic_offset + i] = 0;
        }
        
        let calculated_mic = hmac_sha256(kck, &frame_copy);
        
        &calculated_mic[..WPA_MIC_LEN] == original_mic
    }
}

pub struct SaeHandshake {
    pub state: SaeState,
    pub password: Vec<u8>,
    pub peer_mac: [u8; 6],
    pub own_mac: [u8; 6],
    pub send_confirm: u16,
    pub sync: u16,
    pub scalar: Vec<u8>,
    pub element: Vec<u8>,
    pub peer_scalar: Vec<u8>,
    pub peer_element: Vec<u8>,
    pub pmk: Option<[u8; WPA_PMK_LEN]>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SaeState {
    Nothing,
    Committed,
    Confirmed,
    Accepted,
}

impl SaeHandshake {
    pub fn new(password: &[u8], own_mac: [u8; 6], peer_mac: [u8; 6]) -> Self {
        Self {
            state: SaeState::Nothing,
            password: password.to_vec(),
            peer_mac,
            own_mac,
            send_confirm: 0,
            sync: 0,
            scalar: Vec::new(),
            element: Vec::new(),
            peer_scalar: Vec::new(),
            peer_element: Vec::new(),
            pmk: None,
        }
    }

    pub fn create_commit(&mut self) -> Vec<u8> {
        self.scalar = vec![0u8; 32];
        self.element = vec![0u8; 64];
        generate_random_bytes(&mut self.scalar);
        generate_random_bytes(&mut self.element);
        
        let mut frame = Vec::new();
        frame.push(0x13);
        frame.push(0x00);
        frame.extend_from_slice(&1u16.to_le_bytes());
        frame.extend_from_slice(&19u16.to_le_bytes());
        frame.extend_from_slice(&self.scalar);
        frame.extend_from_slice(&self.element);
        
        self.state = SaeState::Committed;
        frame
    }

    pub fn process_commit(&mut self, frame: &[u8]) -> Result<(), ()> {
        if frame.len() < 100 {
            return Err(());
        }
        
        self.peer_scalar = frame[6..38].to_vec();
        self.peer_element = frame[38..102].to_vec();
        
        if self.state == SaeState::Nothing {
            self.state = SaeState::Committed;
        }
        
        Ok(())
    }

    pub fn create_confirm(&mut self) -> Vec<u8> {
        self.send_confirm += 1;
        
        let mut frame = Vec::new();
        frame.push(0x13);
        frame.push(0x00);
        frame.extend_from_slice(&2u16.to_le_bytes());
        frame.extend_from_slice(&self.send_confirm.to_le_bytes());
        
        let confirm = self.calculate_confirm();
        frame.extend_from_slice(&confirm);
        
        self.state = SaeState::Confirmed;
        frame
    }

    pub fn process_confirm(&mut self, frame: &[u8]) -> Result<(), ()> {
        if frame.len() < 38 {
            return Err(());
        }
        
        let _peer_confirm = &frame[6..38];
        
        self.derive_pmk();
        self.state = SaeState::Accepted;
        
        Ok(())
    }

    fn calculate_confirm(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.send_confirm.to_le_bytes());
        data.extend_from_slice(&self.scalar);
        data.extend_from_slice(&self.element);
        data.extend_from_slice(&self.peer_scalar);
        data.extend_from_slice(&self.peer_element);
        
        hmac_sha256(&self.password, &data)
    }

    fn derive_pmk(&mut self) {
        let mut pmk = [0u8; WPA_PMK_LEN];
        let mut data = Vec::new();
        data.extend_from_slice(&self.scalar);
        data.extend_from_slice(&self.peer_scalar);
        
        let key = hmac_sha256(&self.password, &data);
        pmk.copy_from_slice(&key[..WPA_PMK_LEN]);
        
        self.pmk = Some(pmk);
    }

    pub fn get_pmk(&self) -> Option<[u8; WPA_PMK_LEN]> {
        self.pmk
    }
}

fn pbkdf2_sha256(password: &[u8], salt: &[u8], iterations: u32, output: &mut [u8]) {
    let mut counter = 1u32;
    let mut pos = 0;
    
    while pos < output.len() {
        let mut block = hmac_sha256(password, salt);
        let mut u = block.clone();
        
        for _ in 1..iterations {
            u = hmac_sha256(password, &u);
            for (b, u_byte) in block.iter_mut().zip(u.iter()) {
                *b ^= u_byte;
            }
        }
        
        let copy_len = core::cmp::min(32, output.len() - pos);
        output[pos..pos + copy_len].copy_from_slice(&block[..copy_len]);
        
        pos += copy_len;
        counter += 1;
    }
}

fn prf_sha256(key: &[u8], label: &[u8], data: &[u8], output: &mut [u8]) {
    let mut counter = 0u8;
    let mut pos = 0;
    
    while pos < output.len() {
        let mut input = Vec::new();
        input.extend_from_slice(label);
        input.push(0);
        input.extend_from_slice(data);
        input.push(counter);
        
        let hash = hmac_sha256(key, &input);
        let copy_len = core::cmp::min(32, output.len() - pos);
        output[pos..pos + copy_len].copy_from_slice(&hash[..copy_len]);
        
        pos += copy_len;
        counter += 1;
    }
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut result = vec![0u8; 32];
    
    let mut i_key_pad = vec![0x36u8; 64];
    let mut o_key_pad = vec![0x5cu8; 64];
    
    for (i, &k) in key.iter().enumerate().take(64) {
        i_key_pad[i] ^= k;
        o_key_pad[i] ^= k;
    }
    
    result
}

fn generate_random_bytes(output: &mut [u8]) {
    for byte in output.iter_mut() {
        *byte = (unsafe { core::arch::x86_64::_rdrand64_step(&mut 0) } & 0xFF) as u8;
    }
}

pub struct WpaSupplicant {
    pub version: WpaVersion,
    pub key_mgmt: WpaKeyMgmt,
    pub pairwise_cipher: WpaCipher,
    pub group_cipher: WpaCipher,
    pub handshake: Option<FourWayHandshake>,
    pub sae_handshake: Option<SaeHandshake>,
    pub connected: bool,
}

impl WpaSupplicant {
    pub fn new() -> Self {
        Self {
            version: WpaVersion::Wpa2,
            key_mgmt: WpaKeyMgmt::Wpa2Psk,
            pairwise_cipher: WpaCipher::Ccmp,
            group_cipher: WpaCipher::Ccmp,
            handshake: None,
            sae_handshake: None,
            connected: false,
        }
    }

    pub fn set_wpa2_psk(&mut self, passphrase: &str, ssid: &[u8]) {
        self.version = WpaVersion::Wpa2;
        self.key_mgmt = WpaKeyMgmt::Wpa2Psk;
        let pmk = Pmk::from_passphrase(passphrase, ssid);
        self.handshake = Some(FourWayHandshake::new(pmk));
    }

    pub fn set_wpa3_sae(&mut self, password: &str, own_mac: [u8; 6], peer_mac: [u8; 6]) {
        self.version = WpaVersion::Wpa3;
        self.key_mgmt = WpaKeyMgmt::Wpa3Sae;
        self.sae_handshake = Some(SaeHandshake::new(
            password.as_bytes(),
            own_mac,
            peer_mac,
        ));
    }

    pub fn start_authentication(&mut self) -> Vec<u8> {
        match self.version {
            WpaVersion::Wpa3 => {
                if let Some(sae) = &mut self.sae_handshake {
                    sae.create_commit()
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        }
    }

    pub fn start_4way_handshake(&mut self) -> Vec<u8> {
        if let Some(hs) = &mut self.handshake {
            hs.create_message1()
        } else {
            Vec::new()
        }
    }

    pub fn process_eapol(&mut self, frame: &[u8], aa: &[u8; 6], spa: &[u8; 6]) -> Result<Vec<u8>, ()> {
        if let Some(hs) = &mut self.handshake {
            match hs.state {
                HandshakeState::Message1Sent => {
                    hs.process_message2(frame, aa, spa)?;
                    Ok(hs.create_message3())
                }
                HandshakeState::Message3Sent => {
                    hs.process_message4(frame)?;
                    self.connected = true;
                    Ok(Vec::new())
                }
                _ => Ok(Vec::new()),
            }
        } else {
            Err(())
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn get_ptk(&self) -> Option<&Ptk> {
        self.handshake.as_ref().and_then(|hs| hs.ptk.as_ref())
    }

    pub fn get_gtk(&self) -> Option<&Gtk> {
        self.handshake.as_ref().and_then(|hs| hs.gtk.as_ref())
    }
}
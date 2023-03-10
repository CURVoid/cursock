use std::time::Duration;

#[cfg(any(target_os = "windows", target_os = "linux"))]
use std::ffi::CString;

use crate::*;

/// Struct for raw socketing
///
/// # Examples
/// ```
/// #[cfg(target_os = "linux")]
/// let socket = cursock::Socket::new("wlan0", true).expect("initialize error"); // Linux
/// #[cfg(target_os = "windows")]
/// let socket = cursock::Socket::new("{D37YDFA1-7F4F-F09E-V622-5PACEF22AE49}", true).expect("initialize error"); // Windows
/// // Since windows socket implementation is using npcap you should pass "npcap-like" interface
///
/// let buffer: [u8; 1024] = [0; 1024];
///
/// socket.send_raw_packet(&buffer, true).expect("send error");
///
/// socket.destroy()
/// ```
pub struct Socket {
    #[cfg(target_os = "linux")]
    ifindex: i32,
    #[cfg(target_os = "linux")]
    socket: i32,
    #[cfg(target_os = "windows")]
    adapter: usize,
    src_ip: Ipv4,
    src_mac: Mac,
}

impl Socket {
    /// Initializes socket structure
    ///
    /// # Examples
    /// ```
    /// #[cfg(target_os = "linux")]
    /// let socket = cursock::Socket::new("wlan0", true).expect("initialize error"); // Linux
    /// #[cfg(target_os = "windows")]
    /// let socket = cursock::Socket::new("{D37YDFA1-7F4F-F09E-V622-5PACEF22AE49}", true).expect("initialize error"); // Windows
    /// // Since windows socket implementation is using npcap you should pass "npcap-like" interface
    /// ```
    pub fn new(interface: &str, debug: bool) -> Result<Self, CursedErrorHandle> {
        #[cfg(target_os = "linux")]
        {
            Self::new_linux(interface, debug)
        }
        #[cfg(target_os = "windows")]
        {
            Self::new_windows(interface, debug)
        }

        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let _ = debug;
            let _ = interface;
            Err(CursedErrorHandle::new(
                CursedError::OS,
                format!("{} is not supported yet!", std::env::consts::OS),
            ))
        }
    }
    /// Sends raw packet
    ///
    /// # Examples
    /// ```
    /// let socket = cursock::Socket::new("wlan0", true).expect("initialize error");
    /// let buffer = [0; 100];
    /// socket.send_raw_packet(&buffer, true).expect("send error")
    /// ```
    pub fn send_raw_packet(&self, buffer: &[u8], debug: bool) -> Result<(), CursedErrorHandle> {
        #[cfg(target_os = "linux")]
        {
            self.send_raw_packet_linux(buffer, debug)
        }
        #[cfg(target_os = "windows")]
        {
            self.send_raw_packet_windows(buffer, debug)
        }

        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let _ = buffer;
            let _ = debug;
            Err(CursedErrorHandle::new(
                CursedError::OS,
                format!("{} is not supported yet!", std::env::consts::OS),
            ))
        }
    }
    /// Reads raw packet, can be used for sniffing
    ///
    /// # Examples
    /// ```
    /// let socket = cursock::Socket::new("wlan0", true).expect("initialize error");
    /// let mut buffer = [0; 1000];
    /// socket.read_raw_packet(&mut buffer, true).expect("read error")
    /// ```
    pub fn read_raw_packet(&self, buffer: &mut [u8], debug: bool) -> Result<(), CursedErrorHandle> {
        #[cfg(target_os = "linux")]
        {
            self.read_raw_packet_linux(buffer, debug)
        }
        #[cfg(target_os = "windows")]
        {
            self.read_raw_packet_windows(buffer, debug)
        }

        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            let _ = buffer;
            let _ = debug;
            Err(CursedErrorHandle::new(
                CursedError::OS,
                format!("{} is not supported yet!", std::env::consts::OS),
            ))
        }
    }
    pub fn read_raw_packet_timeout(
        &self,
        buffer: &mut [u8],
        debug: bool,
        timeout: Duration,
    ) -> Result<(), CursedErrorHandle> {
        match Self::read_timeout(Wrapper::new(self), Wrapper::new(buffer), debug, timeout) {
            Some(result) => result,
            None => return Err(
                CursedErrorHandle::new(CursedError::TimeOut, String::from("socket read timed out!"))
            ),
        }
    }

    timeout!{
        read_timeout(
            socket: Wrapper<Socket> => Wrapper::reference,
            buffer: Wrapper<[u8]> => Wrapper::mut_reference,
            debug: bool
        ) -> Result<(), CursedErrorHandle>,
        Self::read_raw_packet
    }   

    pub fn get_src_ip(&self) -> &Ipv4 {
        &self.src_ip
    }
    pub fn get_src_mac(&self) -> &Mac {
        &self.src_mac
    }
    /// Destroys socket structure
    ///
    /// # Examples
    /// ```
    /// let socket = cursock::Socket::new("wlan0", true).expect("initialize error");
    /// socket.destroy()
    /// ```
    pub fn destroy(&self) {
        #[cfg(target_os = "linux")]
        {
            self.destroy_linux()
        }
    }
    #[cfg(target_os = "linux")]
    fn new_linux(interface: &str, debug: bool) -> Result<Self, CursedErrorHandle> {
        let ifname: CString = match CString::new(interface) {
            Ok(ifname) => ifname,
            Err(err) => {
                return Err(CursedErrorHandle::new(
                    CursedError::Parse,
                    format!(
                        "{} is not valid c string can\'t convert it due to {}",
                        interface,
                        err.to_string()
                    ),
                ))
            }
        };

        let socket: i32 = unsafe {
            ccs::socket(
                ccs::AF_PACKET,
                ccs::SOCK_RAW,
                ccs::htons(ccs::ETH_P_ALL as u16) as i32,
            )
        };

        if socket < 0 {
            if debug {
                unsafe { ccs::perror(EMPTY_ARRAY.as_ptr()) }
            }
            return Err(CursedErrorHandle::new(
                CursedError::Initialize,
                format!("Can\'t initialize socket ({} < 0)", socket),
            ));
        }

        let (ifindex, src_ip, src_mac): (i32, Ipv4, Mac) =
            match get_interface_info(socket, ifname.clone(), debug) {
                Ok(ifinfo) => ifinfo,
                Err(err) => return Err(err),
            };

        if debug {
            println!(
                "{} - {}, ip: {}, mac: {}",
                ifindex,
                str_from_bytes(ifname.as_bytes()),
                src_ip,
                src_mac
            );
        }

        Ok(Self {
            socket,
            src_mac,
            src_ip,
            ifindex,
        })
    }
    #[cfg(target_os = "windows")]
    fn new_windows(interface: &str, debug: bool) -> Result<Self, CursedErrorHandle> {
        let (src_ip, src_mac): (Ipv4, Mac) = match get_interface_info(interface) {
            Ok(info) => info,
            Err(err) => return Err(err),
        };

        if debug {
            println!("{} - ip: {}, mac: {}", interface, src_ip, src_mac);
        }

        let pcap_interface: String = format!("rpcap://\\Device\\NPF_{}", interface);
        let pcap_interface: CString = match CString::new(pcap_interface.clone()) {
            Ok(pcap_interface) => pcap_interface,
            Err(err) => {
                return Err(CursedErrorHandle::new(
                    CursedError::Parse,
                    format!(
                        "{} is not valid c string can\'t convert it due to {}",
                        pcap_interface,
                        err.to_string()
                    ),
                ))
            }
        };

        let mut error_buffer: [i8; 256] = [0; 256];

        let adapter: *mut ccs::pcap = unsafe {
            ccs::pcap_open(
                pcap_interface.as_ptr(),
                65535,
                ccs::PCAP_OPENFLAG_PROMISCUOUS,
                1,
                ccs::null_mut(),
                error_buffer.as_mut_ptr(),
            )
        };

        if adapter as usize == 0 {
            return Err(CursedErrorHandle::new(
                CursedError::Sockets,
                format!(
                    "Can\'t open adapted due to {}",
                    str_from_cstr(error_buffer.as_ptr())
                ),
            ));
        }

        Ok(Self {
            adapter: adapter as usize,
            src_ip,
            src_mac,
        })
    }
    #[cfg(target_os = "linux")]
    fn read_raw_packet_linux(
        &self,
        buffer: &mut [u8],
        debug: bool,
    ) -> Result<(), CursedErrorHandle> {
        let length: isize = unsafe {
            ccs::recvfrom(
                self.socket,
                buffer.as_mut_ptr() as *mut std::os::raw::c_void,
                buffer.len(),
                0,
                ccs::null_mut(),
                ccs::null_mut(),
            )
        };

        if length < 0 {
            if debug {
                unsafe { ccs::perror(EMPTY_ARRAY.as_ptr()) }
            }

            return Err(CursedErrorHandle::new(
                CursedError::Sockets,
                String::from("Can\'t receive packet"),
            ));
        }

        if debug {
            println!("Received {} bytes", length);
        }

        Ok(())
    }
    #[cfg(target_os = "windows")]
    fn read_raw_packet_windows(
        &self,
        buffer: &mut [u8],
        debug: bool,
    ) -> Result<(), CursedErrorHandle> {
        let mut header: *mut ccs::pcap_pkthdr = ccs::null_mut();
        let mut pkt_data: *const u8 = ccs::null();

        let result: i32 = unsafe {
            ccs::pcap_next_ex(self.adapter as *mut ccs::pcap, &mut header, &mut pkt_data)
        };

        if result == 0 {
            return Err(CursedErrorHandle::new(
                CursedError::TimeOut,
                String::from("reading raw packet timed out"),
            ));
        }

        let header: &mut ccs::pcap_pkthdr = unsafe { &mut *header };

        if debug {
            println!("Received {} bytes", header.caplen)
        }

        let size: usize = if buffer.len() < header.caplen as usize {
            buffer.len()
        } else {
            header.caplen as usize
        };

        memcpy(buffer.as_mut_ptr(), pkt_data, size);

        Ok(())
    }
    #[cfg(target_os = "windows")]
    fn send_raw_packet_windows(&self, buffer: &[u8], debug: bool) -> Result<(), CursedErrorHandle> {
        let length: i32 = unsafe {
            ccs::pcap_inject(
                self.adapter as *mut ccs::pcap,
                buffer.as_ptr() as *const std::os::raw::c_void,
                buffer.len(),
            )
        };

        if length < 0 {
            let error: String =
                unsafe { str_from_cstr(ccs::pcap_geterr(self.adapter as *mut ccs::pcap)) };

            return Err(CursedErrorHandle::new(
                CursedError::Sockets,
                format!("Can\'t send buffer due to \"{}\"", error),
            ));
        }

        if debug {
            println!("Sended {} bytes", length)
        }

        Ok(())
    }
    #[cfg(target_os = "linux")]
    fn send_raw_packet_linux(&self, buffer: &[u8], debug: bool) -> Result<(), CursedErrorHandle> {
        let raw_src_mac: [u8; MAC_LEN] = self.src_mac.to();
        let mut addr: ccs::sockaddr_ll = ccs::sockaddr_ll {
            sll_family: 0,
            sll_protocol: 0,
            sll_ifindex: self.ifindex,
            sll_hatype: 0,
            sll_pkttype: 0,
            sll_halen: MAC_LEN as u8,
            sll_addr: [0; 8],
        };
        for i in 0..MAC_LEN {
            addr.sll_addr[i] = raw_src_mac[i]
        }

        let addrlen: ccs::SocklenT = std::mem::size_of_val(&addr) as ccs::SocklenT;

        let length: isize = unsafe {
            ccs::sendto(
                self.socket,
                buffer.as_ptr() as *const std::os::raw::c_void,
                buffer.len(),
                0,
                &addr as *const ccs::sockaddr_ll as *const ccs::sockaddr,
                addrlen,
            )
        };

        if length < 0 {
            if debug {
                unsafe { ccs::perror(EMPTY_ARRAY.as_ptr()) }
            }
            return Err(CursedErrorHandle::new(
                CursedError::Sockets,
                String::from("Can\'t send buffer"),
            ));
        }

        if debug {
            println!("Sended {} bytes", length)
        }

        Ok(())
    }
    #[cfg(target_os = "linux")]
    fn destroy_linux(&self) {
        unsafe { ccs::close(self.socket) };
    }
}

#[cfg(target_os = "linux")]
fn get_interface_info(
    socket: i32,
    if_name: CString,
    debug: bool,
) -> Result<(i32, Ipv4, Mac), CursedErrorHandle> {
    let ifru: ccs::ifreq_data = ccs::ifreq_data { ifru_ifindex: 0 };
    let mut if_request: ccs::ifreq = ccs::ifreq {
        ifr_name: [0; 16],
        ifr_ifru: ifru,
    };

    memcpy(
        if_request.ifr_name.as_mut_ptr(),
        if_name.as_ptr(),
        if_name.as_bytes_with_nul().len(),
    );

    let ifindex: i32 = match get_if_index(socket, &mut if_request, debug) {
        Ok(ifindex) => ifindex,
        Err(err) => return Err(err),
    };

    let ip: Ipv4 = match get_if_ip(socket, &mut if_request, debug) {
        Ok(ip) => ip,
        Err(err) => return Err(err),
    };

    let mac: Mac = match get_if_mac(socket, &mut if_request, debug) {
        Ok(mac) => mac,
        Err(err) => return Err(err),
    };

    Ok((ifindex, ip, mac))
}

#[cfg(target_os = "linux")]
fn get_if_index(socket: i32, ifr: *mut ccs::ifreq, debug: bool) -> Result<i32, CursedErrorHandle> {
    let err: i32 = unsafe { ccs::ioctl(socket, ccs::SIOCGIFINDEX, ifr) };

    if err == -1 {
        if debug {
            unsafe { ccs::perror(EMPTY_ARRAY.as_ptr()) }
        }
        return Err(CursedErrorHandle::new(
            CursedError::Sockets,
            String::from("Got error while getting SIOCGIFINDEX"),
        ));
    }

    let index: i32 = unsafe { (*ifr).ifr_ifru.ifru_ifindex.clone() };

    Ok(index)
}

#[cfg(target_os = "linux")]
fn get_if_ip(socket: i32, ifr: *mut ccs::ifreq, debug: bool) -> Result<Ipv4, CursedErrorHandle> {
    let err: i32;

    err = unsafe { ccs::ioctl(socket, ccs::SIOCGIFADDR, ifr) };

    if err == -1 {
        if debug {
            unsafe { ccs::perror(EMPTY_ARRAY.as_ptr()) }
        }
        return Err(CursedErrorHandle::new(
            CursedError::Sockets,
            String::from("Got error while getting SIOCGIFADDR"),
        ));
    }

    let addr: *const ccs::sockaddr_in =
        unsafe { &(*ifr).ifr_ifru.ifru_addr as *const ccs::sockaddr } as *const ccs::sockaddr_in;
    let mut ip: [u8; IPV4_LEN] = [0; IPV4_LEN];

    memcpy(
        ip.as_mut_ptr(),
        unsafe { &(*addr).sin_addr.s_addr },
        std::mem::size_of::<[u8; IPV4_LEN]>(),
    );

    Ok(Handle::from(ip))
}

#[cfg(target_os = "linux")]
fn get_if_mac(socket: i32, ifr: *mut ccs::ifreq, debug: bool) -> Result<Mac, CursedErrorHandle> {
    let err: i32 = unsafe { ccs::ioctl(socket, ccs::SIOCGIFHWADDR, ifr) };

    if err == -1 {
        if debug {
            unsafe { ccs::perror(EMPTY_ARRAY.as_ptr()) }
        }
        return Err(CursedErrorHandle::new(
            CursedError::Sockets,
            String::from("Got error while getting SIOCGIFHWADDR"),
        ));
    }

    let sa_data: [i8; 14] = unsafe { (*ifr).ifr_ifru.ifru_hwaddr.sa_data };

    let mut mac: [u8; MAC_LEN] = [0; MAC_LEN];

    memcpy(
        mac.as_mut_ptr(),
        sa_data.as_ptr(),
        std::mem::size_of::<[u8; MAC_LEN]>(),
    );

    Ok(Handle::from(mac))
}

#[cfg(target_os = "windows")]
fn get_interface_info(adapter_name: &str) -> Result<(Ipv4, Mac), CursedErrorHandle> {
    let mut size: u32 = 0;

    unsafe { ccs::GetAdaptersInfo(ccs::null_mut(), &mut size) };

    let mut buffer: Vec<u8> = vec![0; size as usize];
    let p_adapter_info: *mut ccs::IP_ADAPTER_INFO =
        buffer.as_mut_ptr() as *mut ccs::IP_ADAPTER_INFO;
    let result: u32 = unsafe { ccs::GetAdaptersInfo(p_adapter_info, &mut size) };

    if result != 0 {
        return Err(CursedErrorHandle::new(
            CursedError::Sockets,
            format!("Got {} error while getting adapters info", result),
        ));
    }

    let mut adapter: *mut ccs::IP_ADAPTER_INFO = p_adapter_info;
    let mut adapter_info: Option<(Ipv4, Mac)> = None;

    loop {
        if adapter as usize == 0 {
            break;
        }
        let adapter_ref: &mut ccs::IP_ADAPTER_INFO = unsafe { &mut *adapter };

        if adapter_name == &str_from_cstr(adapter_ref.adaptername.as_ptr())[..] {
            let mut mac_addr: [u8; MAC_LEN] = [0; MAC_LEN];
            memcpy(
                mac_addr.as_mut_ptr(),
                adapter_ref.address.as_ptr(),
                std::mem::size_of::<[u8; MAC_LEN]>(),
            );

            let mut ip_addr: [u8; IPV4_LEN] = [0; IPV4_LEN];
            memcpy(
                &mut ip_addr,
                &adapter_ref.ipaddresslist.context,
                std::mem::size_of::<[u8; IPV4_LEN]>(),
            );

            adapter_info = Some((Handle::from(ip_addr), Handle::from(mac_addr)))
        }

        adapter = adapter_ref.next
    }
    let adapter_info: (Ipv4, Mac) = match adapter_info {
        Some(adapter_info) => adapter_info,
        None => {
            return Err(CursedErrorHandle::new(
                CursedError::InvalidArgument,
                format!("{} is not valid adapter name", adapter_name),
            ))
        }
    };

    Ok(adapter_info)
}

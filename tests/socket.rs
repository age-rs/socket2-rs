#![allow(clippy::bool_assert_comparison)]
#[cfg(all(
    feature = "all",
    any(
        target_os = "android",
        target_os = "freebsd",
        target_os = "ios",
        target_os = "visionos",
        target_os = "linux",
        target_os = "macos",
        target_os = "tvos",
        target_os = "watchos",
    )
))]
use std::fs::File;
use std::io;
#[cfg(not(any(target_os = "redox", target_os = "vita")))]
use std::io::IoSlice;
use std::io::Read;
use std::io::Write;
#[cfg(not(target_os = "vita"))]
use std::mem::MaybeUninit;
use std::mem::{self};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};
#[cfg(not(any(target_os = "redox", target_os = "vita")))]
use std::net::{Ipv6Addr, SocketAddrV6};
#[cfg(all(
    feature = "all",
    any(
        target_os = "android",
        target_os = "freebsd",
        target_os = "ios",
        target_os = "visionos",
        target_os = "linux",
        target_os = "macos",
        target_os = "tvos",
        target_os = "watchos",
    )
))]
use std::num::NonZeroUsize;
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
#[cfg(windows)]
use std::os::windows::io::AsRawSocket;
#[cfg(unix)]
use std::path::Path;
use std::str;
#[cfg(not(target_os = "vita"))]
use std::thread;
use std::time::Duration;
use std::{env, fs};

#[cfg(windows)]
use windows_sys::Win32::Foundation::{GetHandleInformation, HANDLE_FLAG_INHERIT};

#[cfg(not(any(target_os = "redox", target_os = "vita")))]
use socket2::MaybeUninitSlice;
#[cfg(not(target_os = "vita"))]
use socket2::TcpKeepalive;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};

#[test]
fn domain_for_address() {
    let ipv4: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    assert!(ipv4.is_ipv4());
    let ipv6: SocketAddr = "[::1]:8080".parse().unwrap();
    assert!(ipv6.is_ipv6());

    assert_eq!(Domain::for_address(ipv4), Domain::IPV4);
    assert_eq!(Domain::for_address(ipv6), Domain::IPV6);
}

#[test]
fn domain_fmt_debug() {
    let tests = &[
        (Domain::IPV4, "AF_INET"),
        (Domain::IPV6, "AF_INET6"),
        (Domain::UNIX, "AF_UNIX"),
        #[cfg(all(feature = "all", any(target_os = "fuchsia", target_os = "linux")))]
        (Domain::PACKET, "AF_PACKET"),
        #[cfg(all(feature = "all", any(target_os = "android", target_os = "linux")))]
        (Domain::VSOCK, "AF_VSOCK"),
        (0.into(), "AF_UNSPEC"),
        (500.into(), "500"),
    ];

    let mut buf = Vec::new();
    for (input, want) in tests {
        buf.clear();
        write!(buf, "{input:?}").unwrap();
        let got = str::from_utf8(&buf).unwrap();
        assert_eq!(got, *want);
    }
}

#[test]
fn type_fmt_debug() {
    let tests = &[
        (Type::STREAM, "SOCK_STREAM"),
        (Type::DGRAM, "SOCK_DGRAM"),
        #[cfg(feature = "all")]
        (Type::SEQPACKET, "SOCK_SEQPACKET"),
        #[cfg(all(feature = "all", not(target_os = "redox")))]
        (Type::RAW, "SOCK_RAW"),
        (500.into(), "500"),
    ];

    let mut buf = Vec::new();
    for (input, want) in tests {
        buf.clear();
        write!(buf, "{input:?}").unwrap();
        let got = str::from_utf8(&buf).unwrap();
        assert_eq!(got, *want);
    }
}

#[test]
fn protocol_fmt_debug() {
    let tests = &[
        (Protocol::ICMPV4, "IPPROTO_ICMP"),
        (Protocol::ICMPV6, "IPPROTO_ICMPV6"),
        (Protocol::TCP, "IPPROTO_TCP"),
        (Protocol::UDP, "IPPROTO_UDP"),
        #[cfg(target_os = "linux")]
        (Protocol::MPTCP, "IPPROTO_MPTCP"),
        #[cfg(all(feature = "all", target_os = "linux"))]
        (Protocol::DCCP, "IPPROTO_DCCP"),
        #[cfg(all(feature = "all", any(target_os = "freebsd", target_os = "linux")))]
        (Protocol::SCTP, "IPPROTO_SCTP"),
        (500.into(), "500"),
    ];

    let mut buf = Vec::new();
    for (input, want) in tests {
        buf.clear();
        write!(buf, "{input:?}").unwrap();
        let got = str::from_utf8(&buf).unwrap();
        assert_eq!(got, *want);
    }
}

#[test]
#[should_panic]
#[cfg(unix)]
fn from_invalid_raw_fd_should_panic() {
    use std::os::unix::io::FromRawFd;
    let _socket = unsafe { Socket::from_raw_fd(-1) };
}

#[test]
fn socket_address_unix() {
    let string = "/tmp/socket";
    let addr = SockAddr::unix(string).unwrap();
    assert!(addr.as_socket_ipv4().is_none());
    assert!(addr.as_socket_ipv6().is_none());
    assert!(!addr.is_ipv4());
    assert!(!addr.is_ipv6());
    assert!(addr.is_unix());
    assert_eq!(addr.domain(), Domain::UNIX);
    #[cfg(unix)]
    {
        assert!(!addr.is_unnamed());
        assert_eq!(addr.as_pathname(), Some(Path::new(string)));
        assert_eq!(addr.as_abstract_namespace(), None);
        let unix = addr.as_unix().unwrap();
        assert_eq!(addr.as_pathname(), unix.as_pathname());
    }
}

#[test]
fn socket_address_unix_unnamed() {
    let addr = SockAddr::unix("").unwrap();
    assert!(addr.as_socket_ipv4().is_none());
    assert!(addr.as_socket_ipv6().is_none());
    assert!(!addr.is_ipv4());
    assert!(!addr.is_ipv6());
    assert!(addr.is_unix());
    assert_eq!(addr.domain(), Domain::UNIX);
    #[cfg(unix)]
    {
        assert!(addr.is_unnamed());
        assert_eq!(addr.as_pathname(), None);
        assert_eq!(addr.as_abstract_namespace(), None);
        assert!(addr.as_unix().is_none());
    }
}

#[test]
#[cfg(all(
    any(target_os = "linux", target_os = "android", target_os = "cygwin"),
    feature = "all",
))]
fn socket_address_unix_abstract_namespace() {
    let path = "\0h".repeat(108 / 2);
    let addr = SockAddr::unix(&path).unwrap();
    assert_eq!(
        addr.len() as usize,
        std::mem::size_of::<libc::sockaddr_un>()
    );
    assert!(!addr.is_unnamed());
    // The first byte is the opening null bytes of an abstract address, should not be included.
    assert_eq!(addr.as_abstract_namespace(), Some(&path.as_bytes()[1..]));
    assert!(addr.as_pathname().is_none());
    assert!(!addr.is_unnamed());
}

#[test]
#[cfg(all(feature = "all", any(target_os = "android", target_os = "linux")))]
fn socket_address_vsock() {
    let addr = SockAddr::vsock(1, 9999);
    assert!(addr.as_socket_ipv4().is_none());
    assert!(addr.as_socket_ipv6().is_none());
    assert_eq!(addr.as_vsock_address().unwrap(), (1, 9999));
}

#[test]
fn set_nonblocking() {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    assert_nonblocking(&socket, false);

    socket.set_nonblocking(true).unwrap();
    assert_nonblocking(&socket, true);

    socket.set_nonblocking(false).unwrap();
    assert_nonblocking(&socket, false);
}

fn assert_common_flags(socket: &Socket, expected: bool) {
    #[cfg(all(unix, not(target_os = "vita")))]
    assert_close_on_exec(socket, expected);
    #[cfg(any(
        target_os = "ios",
        target_os = "visionos",
        target_os = "macos",
        target_os = "tvos",
        target_os = "watchos",
    ))]
    assert_flag_no_sigpipe(socket, expected);
    #[cfg(windows)]
    assert_flag_no_inherit(socket, expected);

    // Vita does not have process API, so neither SO_NOSIGPIPE nor FD_CLOEXEC are supported on this platform
    #[cfg(target_os = "vita")]
    {
        let _ = socket;
        let _ = expected;
    }
}

#[test]
fn common_flags() {
    let listener = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    assert_common_flags(&listener, true);

    listener.bind(&any_ipv4()).unwrap();
    listener.listen(1).unwrap();

    let client = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    client.connect(&listener.local_addr().unwrap()).unwrap();

    let accepted = listener.accept().unwrap().0;
    assert_common_flags(&accepted, true);
}

#[test]
fn no_common_flags() {
    let listener = Socket::new_raw(Domain::IPV4, Type::STREAM, None).unwrap();
    assert_common_flags(&listener, false);

    listener.bind(&any_ipv4()).unwrap();
    listener.listen(1).unwrap();

    let client = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    client.connect(&listener.local_addr().unwrap()).unwrap();

    let accepted = listener.accept_raw().unwrap().0;
    assert_common_flags(&accepted, false);
}

#[cfg(all(
    feature = "all",
    any(
        target_os = "android",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "fuchsia",
        target_os = "linux",
        target_os = "netbsd",
        target_os = "openbsd"
    )
))]
#[test]
fn type_nonblocking() {
    let ty = Type::STREAM.nonblocking();
    let socket = Socket::new(Domain::IPV4, ty, None).unwrap();
    assert_nonblocking(&socket, true);
}

/// Assert that `NONBLOCK` is set on `socket`.
#[cfg(unix)]
#[track_caller]
pub fn assert_nonblocking(socket: &Socket, want: bool) {
    #[cfg(all(feature = "all", unix))]
    assert_eq!(socket.nonblocking().unwrap(), want, "non-blocking option");

    #[cfg(not(any(all(feature = "all", unix), target_os = "vita")))]
    {
        let flags = unsafe { libc::fcntl(socket.as_raw_fd(), libc::F_GETFL) };
        assert_eq!(flags & libc::O_NONBLOCK != 0, want, "non-blocking option");
    }

    #[cfg(all(target_os = "vita", not(feature = "all")))]
    {
        let mut optval: libc::c_int = 0;
        let mut optlen = std::mem::size_of::<libc::c_int>() as libc::socklen_t;

        let res = unsafe {
            libc::getsockopt(
                socket.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_NONBLOCK,
                &mut optval as *mut libc::c_int as _,
                &mut optlen,
            )
        };
        assert_eq!(res, 0, "unable to get non-blocing option");
        assert_eq!(optval > 0, want, "non-blocking option");
    }
}

#[cfg(windows)]
#[track_caller]
pub fn assert_nonblocking(_: &Socket, _: bool) {
    // No way to get this information...
}

#[cfg(all(unix, feature = "all", not(target_os = "vita")))]
#[test]
fn set_cloexec() {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    assert_close_on_exec(&socket, true);

    socket.set_cloexec(false).unwrap();
    assert_close_on_exec(&socket, false);

    socket.set_cloexec(true).unwrap();
    assert_close_on_exec(&socket, true);
}

#[cfg(all(
    feature = "all",
    any(
        target_os = "android",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "fuchsia",
        target_os = "hurd",
        target_os = "linux",
        target_os = "netbsd",
        target_os = "openbsd"
    )
))]
#[test]
fn type_cloexec() {
    let ty = Type::STREAM.cloexec();
    let socket = Socket::new(Domain::IPV4, ty, None).unwrap();
    assert_close_on_exec(&socket, true);
}

/// Assert that `CLOEXEC` is set on `socket`.
#[cfg(unix)]
#[track_caller]
pub fn assert_close_on_exec<S>(socket: &S, want: bool)
where
    S: AsRawFd,
{
    let flags = unsafe { libc::fcntl(socket.as_raw_fd(), libc::F_GETFD) };
    assert_eq!(flags & libc::FD_CLOEXEC != 0, want, "CLOEXEC option");
}

#[cfg(all(feature = "all", windows))]
#[test]
fn set_no_inherit() {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    assert_flag_no_inherit(&socket, true);

    socket.set_no_inherit(false).unwrap();
    assert_flag_no_inherit(&socket, false);

    socket.set_no_inherit(true).unwrap();
    assert_flag_no_inherit(&socket, true);
}

#[cfg(all(feature = "all", windows))]
#[test]
fn type_no_inherit() {
    let ty = Type::STREAM.no_inherit();
    let socket = Socket::new(Domain::IPV4, ty, None).unwrap();
    assert_flag_no_inherit(&socket, true);
}

/// Assert that `FLAG_INHERIT` is not set on `socket`.
#[cfg(windows)]
#[track_caller]
pub fn assert_flag_no_inherit<S>(socket: &S, want: bool)
where
    S: AsRawSocket,
{
    let mut flags = 0;
    if unsafe { GetHandleInformation(socket.as_raw_socket() as _, &mut flags) } == 0 {
        let err = io::Error::last_os_error();
        panic!("unexpected error: {err}");
    }
    assert_eq!(
        flags & HANDLE_FLAG_INHERIT != 0,
        !want,
        "FLAG_INHERIT option"
    );
}

#[cfg(all(
    feature = "all",
    any(
        target_os = "ios",
        target_os = "visionos",
        target_os = "macos",
        target_os = "tvos",
        target_os = "watchos",
    )
))]
#[test]
fn set_nosigpipe() {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    assert_flag_no_sigpipe(&socket, true);

    socket.set_nosigpipe(false).unwrap();
    assert_flag_no_sigpipe(&socket, false);

    socket.set_nosigpipe(true).unwrap();
    assert_flag_no_sigpipe(&socket, true);
}

/// Assert that `SO_NOSIGPIPE` is set on `socket`.
#[cfg(any(
    target_os = "ios",
    target_os = "visionos",
    target_os = "macos",
    target_os = "tvos",
    target_os = "watchos",
))]
#[track_caller]
pub fn assert_flag_no_sigpipe<S>(socket: &S, want: bool)
where
    S: AsRawFd,
{
    use std::mem::size_of;
    let mut flags: libc::c_int = 0;
    let mut length = size_of::<libc::c_int>() as libc::socklen_t;
    let res = unsafe {
        libc::getsockopt(
            socket.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_NOSIGPIPE,
            &mut flags as *mut _ as *mut _,
            &mut length,
        )
    };
    if res != 0 {
        panic!("unexpected error: {}", io::Error::last_os_error());
    }
    assert_eq!(length as usize, size_of::<libc::c_int>());
    assert_eq!(flags, want as _, "non-blocking option");
}

const DATA: &[u8] = b"hello world";

#[test]
fn connect_timeout_unrouteable() {
    // This IP is unroutable, so connections should always time out.
    let addr = "10.255.255.1:80".parse::<SocketAddr>().unwrap().into();

    let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    match socket.connect_timeout(&addr, Duration::from_millis(250)) {
        Ok(_) => panic!("unexpected success"),
        Err(ref err) if err.kind() == io::ErrorKind::TimedOut => {}
        Err(err) => panic!("unexpected error {err}"),
    }
}

#[test]
#[cfg(not(target_os = "vita"))] // Loopback has special behavior on vita
fn connect_timeout_unbound() {
    // Bind and drop a socket to track down a "probably unassigned" port.
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap().into();
    socket.bind(&addr).unwrap();
    let addr = socket.local_addr().unwrap();
    drop(socket);

    let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    match socket.connect_timeout(&addr, Duration::from_millis(250)) {
        Ok(_) => panic!("unexpected success"),
        Err(ref err)
            if err.kind() == io::ErrorKind::ConnectionRefused
                || err.kind() == io::ErrorKind::TimedOut => {}
        Err(err) => panic!("unexpected error {err}"),
    }
}

#[test]
fn connect_timeout_valid() {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    socket
        .bind(&"127.0.0.1:0".parse::<SocketAddr>().unwrap().into())
        .unwrap();
    socket.listen(128).unwrap();

    let addr = socket.local_addr().unwrap();

    let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    socket
        .connect_timeout(&addr, Duration::from_millis(250))
        .unwrap();
}

#[test]
#[cfg(all(feature = "all", unix))]
fn pair() {
    let (mut a, mut b) = Socket::pair(Domain::UNIX, Type::STREAM, None).unwrap();
    let _ = a.write(DATA).unwrap();
    let mut buf = [0; DATA.len() + 1];
    let n = b.read(&mut buf).unwrap();
    assert_eq!(n, DATA.len());
    assert_eq!(&buf[..n], DATA);
}

fn unix_sockets_supported() -> bool {
    #[cfg(windows)]
    {
        // Only some versions of Windows support Unix sockets.
        match Socket::new(Domain::UNIX, Type::STREAM, None) {
            Ok(_) => {}
            Err(err)
                if err.raw_os_error()
                    == Some(windows_sys::Win32::Networking::WinSock::WSAEAFNOSUPPORT as i32) =>
            {
                return false;
            }
            Err(err) => panic!("socket error: {err}"),
        }
    }

    #[cfg(target_os = "vita")]
    return false;

    #[cfg(not(target_os = "vita"))]
    true
}

#[test]
fn unix() {
    if !unix_sockets_supported() {
        return;
    }
    let mut path = env::temp_dir();
    path.push("socket2");
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path.push("unix");

    let addr = SockAddr::unix(path).unwrap();

    let listener = Socket::new(Domain::UNIX, Type::STREAM, None).unwrap();
    #[cfg(target_os = "cygwin")]
    listener.set_no_peercred().unwrap();
    listener.bind(&addr).unwrap();
    listener.listen(10).unwrap();

    let mut a = Socket::new(Domain::UNIX, Type::STREAM, None).unwrap();
    #[cfg(target_os = "cygwin")]
    a.set_no_peercred().unwrap();
    a.connect(&addr).unwrap();
    let mut b = listener.accept().unwrap().0;

    let _ = a.write(DATA).unwrap();
    let mut buf = [0; DATA.len() + 1];
    let n = b.read(&mut buf).unwrap();
    assert_eq!(n, DATA.len());
    assert_eq!(&buf[..n], DATA);
}

#[test]
#[cfg(all(feature = "all", any(target_os = "android", target_os = "linux")))]
#[ignore = "using VSOCK family requires optional kernel support (works when enabled)"]
fn vsock() {
    let addr = SockAddr::vsock(libc::VMADDR_CID_LOCAL, libc::VMADDR_PORT_ANY);

    let listener = Socket::new(Domain::VSOCK, Type::STREAM, None).unwrap();
    listener.bind(&addr).unwrap();
    listener.listen(10).unwrap();

    let (_, port) = listener.local_addr().unwrap().as_vsock_address().unwrap();
    let addr = SockAddr::vsock(libc::VMADDR_CID_LOCAL, port);
    let mut a = Socket::new(Domain::VSOCK, Type::STREAM, None).unwrap();
    a.connect(&addr).unwrap();
    let mut b = listener.accept().unwrap().0;

    let _ = a.write(DATA).unwrap();
    let mut buf = [0; DATA.len() + 1];
    let n = b.read(&mut buf).unwrap();
    assert_eq!(n, DATA.len());
    assert_eq!(&buf[..n], DATA);
}

#[test]
#[cfg(not(target_os = "vita"))] // Vita does not support OOB
fn out_of_band() {
    let listener = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    listener.bind(&any_ipv4()).unwrap();
    listener.listen(1).unwrap();

    let sender = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    sender.bind(&any_ipv4()).unwrap();
    sender.connect(&listener.local_addr().unwrap()).unwrap();

    let (receiver, _) = listener.accept().unwrap();

    sender.send(DATA).unwrap();

    const FIRST: &[u8] = b"!";
    assert_eq!(sender.send_out_of_band(FIRST).unwrap(), FIRST.len());
    // On macOS if no `MSG_OOB` is available it will return `EINVAL`, to prevent
    // this from happening we'll sleep to ensure the data is present.
    thread::sleep(Duration::from_millis(10));

    let mut buf = [MaybeUninit::new(1); DATA.len() + 1];
    let n = receiver.recv_out_of_band(&mut buf).unwrap();
    assert_eq!(n, FIRST.len());
    assert_eq!(unsafe { assume_init(&buf[..n]) }, FIRST);

    let n = receiver.recv(&mut buf).unwrap();
    assert_eq!(n, DATA.len());
    assert_eq!(unsafe { assume_init(&buf[..n]) }, DATA);
}

#[test]
#[cfg(not(any(target_os = "redox", target_os = "vita")))]
fn udp_peek_sender() {
    let (socket_a, socket_b) = udp_pair_unconnected();

    let socket_a_addr = socket_a.local_addr().unwrap();
    let socket_b_addr = socket_b.local_addr().unwrap();

    socket_b.send_to(b"Hello, world!", &socket_a_addr).unwrap();

    let sender_addr = socket_a.peek_sender().unwrap();

    assert_eq!(sender_addr.as_socket(), socket_b_addr.as_socket());
}

#[test]
#[cfg(not(any(target_os = "redox", target_os = "vita")))]
fn send_recv_vectored() {
    let (socket_a, socket_b) = udp_pair_connected();

    let sent = socket_a
        .send_vectored(&[
            IoSlice::new(b"the"),
            IoSlice::new(b"weeknight"),
            IoSlice::new(b"would"),
            IoSlice::new(b"yellow"),
        ])
        .unwrap();
    assert_eq!(sent, 23);

    let mut the = [MaybeUninit::new(1); 3];
    let mut wee = [MaybeUninit::new(2); 3];
    let mut knight = [MaybeUninit::new(3); 6];
    let mut would = [MaybeUninit::new(4); 5];
    let mut yell = [MaybeUninit::new(5); 4];
    let mut ow = [MaybeUninit::new(6); 2];

    let (received, flags) = socket_b
        .recv_vectored(&mut [
            MaybeUninitSlice::new(&mut the),
            MaybeUninitSlice::new(&mut wee),
            MaybeUninitSlice::new(&mut knight),
            MaybeUninitSlice::new(&mut would),
            MaybeUninitSlice::new(&mut yell),
            MaybeUninitSlice::new(&mut ow),
        ])
        .unwrap();
    assert_eq!(received, 23);
    #[cfg(all(unix, not(target_os = "redox")))]
    assert_eq!(flags.is_end_of_record(), false);
    #[cfg(all(unix, not(target_os = "redox")))]
    assert_eq!(flags.is_out_of_band(), false);
    assert_eq!(flags.is_truncated(), false);

    assert_eq!(unsafe { assume_init(&the) }, b"the");
    assert_eq!(unsafe { assume_init(&wee) }, b"wee");
    assert_eq!(unsafe { assume_init(&knight) }, b"knight");
    assert_eq!(unsafe { assume_init(&would) }, b"would");
    assert_eq!(unsafe { assume_init(&yell) }, b"yell");
    assert_eq!(unsafe { assume_init(&ow) }, b"ow");
}

#[test]
#[cfg(not(any(target_os = "redox", target_os = "vita")))]
fn send_from_recv_to_vectored() {
    let (socket_a, socket_b) = udp_pair_unconnected();
    let addr_a = socket_a.local_addr().unwrap();
    let addr_b = socket_b.local_addr().unwrap();

    let sent = socket_a
        .send_to_vectored(
            &[
                IoSlice::new(b"surgeon"),
                IoSlice::new(b"has"),
                IoSlice::new(b"menswear"),
            ],
            &addr_b,
        )
        .unwrap();
    assert_eq!(sent, 18);

    let mut surgeon = [MaybeUninit::new(10); 7];
    let mut has = [MaybeUninit::new(11); 3];
    let mut men = [MaybeUninit::new(12); 3];
    let mut swear = [MaybeUninit::new(13); 5];
    let (received, flags, addr) = socket_b
        .recv_from_vectored(&mut [
            MaybeUninitSlice::new(&mut surgeon),
            MaybeUninitSlice::new(&mut has),
            MaybeUninitSlice::new(&mut men),
            MaybeUninitSlice::new(&mut swear),
        ])
        .unwrap();

    assert_eq!(received, 18);
    #[cfg(all(unix, not(target_os = "redox")))]
    assert_eq!(flags.is_end_of_record(), false);
    #[cfg(all(unix, not(target_os = "redox")))]
    assert_eq!(flags.is_out_of_band(), false);
    assert_eq!(flags.is_truncated(), false);
    #[cfg(all(feature = "all", any(target_os = "android", target_os = "linux")))]
    assert_eq!(flags.is_confirm(), false);
    #[cfg(all(feature = "all", any(target_os = "android", target_os = "linux")))]
    assert_eq!(flags.is_dontroute(), false);
    assert_eq!(
        addr.as_socket_ipv6().unwrap(),
        addr_a.as_socket_ipv6().unwrap()
    );

    assert_eq!(unsafe { assume_init(&surgeon) }, b"surgeon");
    assert_eq!(unsafe { assume_init(&has) }, b"has");
    assert_eq!(unsafe { assume_init(&men) }, b"men");
    assert_eq!(unsafe { assume_init(&swear) }, b"swear");
}

#[test]
#[cfg(not(any(target_os = "redox", target_os = "vita")))]
fn sendmsg() {
    let (socket_a, socket_b) = udp_pair_unconnected();

    const DATA: &[u8] = b"Hello, World!";

    let bufs = &[IoSlice::new(DATA)];
    let addr_b = socket_b.local_addr().unwrap();
    let msg = socket2::MsgHdr::new().with_addr(&addr_b).with_buffers(bufs);
    let sent = socket_a.sendmsg(&msg, 0).unwrap();
    assert_eq!(sent, DATA.len());

    let mut buf = Vec::with_capacity(DATA.len() + 1);
    let received = socket_b.recv(buf.spare_capacity_mut()).unwrap();
    assert_eq!(received, DATA.len());
}

#[test]
#[cfg(not(any(target_os = "redox", target_os = "vita")))]
fn recv_vectored_truncated() {
    let (socket_a, socket_b) = udp_pair_connected();

    let sent = socket_a
        .send(b"do not feed the gremlins after midnight")
        .unwrap();
    assert_eq!(sent, 39);

    let mut buffer = [MaybeUninit::new(20); 24];

    let (received, flags) = socket_b
        .recv_vectored(&mut [MaybeUninitSlice::new(&mut buffer)])
        .unwrap();
    assert_eq!(received, 24);
    assert_eq!(flags.is_truncated(), true);
    assert_eq!(unsafe { assume_init(&buffer) }, b"do not feed the gremlins");
}

#[test]
#[cfg(not(any(target_os = "redox", target_os = "vita")))]
fn recv_from_vectored_truncated() {
    let (socket_a, socket_b) = udp_pair_unconnected();
    let addr_a = socket_a.local_addr().unwrap();
    let addr_b = socket_b.local_addr().unwrap();

    let sent = socket_a
        .send_to(b"do not feed the gremlins after midnight", &addr_b)
        .unwrap();
    assert_eq!(sent, 39);

    let mut buffer = [MaybeUninit::new(30); 24];

    let (received, flags, addr) = socket_b
        .recv_from_vectored(&mut [MaybeUninitSlice::new(&mut buffer)])
        .unwrap();
    assert_eq!(received, 24);
    assert_eq!(flags.is_truncated(), true);
    assert_eq!(
        addr.as_socket_ipv6().unwrap(),
        addr_a.as_socket_ipv6().unwrap()
    );
    assert_eq!(unsafe { assume_init(&buffer) }, b"do not feed the gremlins");
}

/// Create a pair of non-connected UDP sockets suitable for unit tests.
#[cfg(not(any(target_os = "redox", target_os = "vita")))]
fn udp_pair_unconnected() -> (Socket, Socket) {
    // Use ephemeral ports assigned by the OS.
    let unspecified_addr = SocketAddrV6::new(Ipv6Addr::LOCALHOST, 0, 0, 0);
    let socket_a = Socket::new(Domain::IPV6, Type::DGRAM, None).unwrap();
    let socket_b = Socket::new(Domain::IPV6, Type::DGRAM, None).unwrap();

    socket_a.bind(&unspecified_addr.into()).unwrap();
    socket_b.bind(&unspecified_addr.into()).unwrap();

    // Set low timeouts to prevent the tests from blocking.
    socket_a
        .set_read_timeout(Some(std::time::Duration::from_millis(10)))
        .unwrap();
    socket_b
        .set_read_timeout(Some(std::time::Duration::from_millis(10)))
        .unwrap();
    socket_a
        .set_write_timeout(Some(std::time::Duration::from_millis(10)))
        .unwrap();
    socket_b
        .set_write_timeout(Some(std::time::Duration::from_millis(10)))
        .unwrap();

    (socket_a, socket_b)
}

/// Create a pair of connected UDP sockets suitable for unit tests.
#[cfg(not(any(target_os = "redox", target_os = "vita")))]
fn udp_pair_connected() -> (Socket, Socket) {
    let (socket_a, socket_b) = udp_pair_unconnected();

    let addr_a = socket_a.local_addr().unwrap();
    let addr_b = socket_b.local_addr().unwrap();
    socket_a.connect(&addr_b).unwrap();
    socket_b.connect(&addr_a).unwrap();

    (socket_a, socket_b)
}

#[test]
#[cfg(not(target_os = "vita"))]
fn tcp_keepalive() {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    let params = TcpKeepalive::new().with_time(Duration::from_secs(200));

    #[cfg(all(
        feature = "all",
        any(
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "fuchsia",
            target_os = "ios",
            target_os = "visionos",
            target_os = "linux",
            target_os = "macos",
            target_os = "netbsd",
            target_os = "tvos",
            target_os = "watchos",
            target_os = "windows",
        )
    ))]
    let params = params.with_interval(Duration::from_secs(30));

    #[cfg(all(
        feature = "all",
        any(
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "fuchsia",
            target_os = "ios",
            target_os = "visionos",
            target_os = "linux",
            target_os = "macos",
            target_os = "netbsd",
            target_os = "tvos",
            target_os = "watchos",
            target_os = "windows"
        )
    ))]
    let params = params.with_retries(10);

    // Set the parameters.
    socket.set_tcp_keepalive(&params).unwrap();

    #[cfg(all(
        feature = "all",
        not(any(windows, target_os = "haiku", target_os = "openbsd"))
    ))]
    assert_eq!(
        socket.tcp_keepalive_time().unwrap(),
        Duration::from_secs(200)
    );

    #[cfg(all(
        feature = "all",
        any(
            target_os = "android",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "fuchsia",
            target_os = "illumos",
            target_os = "ios",
            target_os = "visionos",
            target_os = "linux",
            target_os = "macos",
            target_os = "netbsd",
            target_os = "tvos",
            target_os = "watchos",
        )
    ))]
    assert_eq!(
        socket.tcp_keepalive_interval().unwrap(),
        Duration::from_secs(30)
    );

    #[cfg(all(
        feature = "all",
        any(
            target_os = "android",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "fuchsia",
            target_os = "illumos",
            target_os = "ios",
            target_os = "visionos",
            target_os = "linux",
            target_os = "macos",
            target_os = "netbsd",
            target_os = "tvos",
            target_os = "watchos",
            target_os = "windows",
        )
    ))]
    assert_eq!(socket.tcp_keepalive_retries().unwrap(), 10);
}

#[cfg(all(feature = "all", any(target_os = "fuchsia", target_os = "linux")))]
#[test]
#[ignore = "setting `SO_BINDTODEVICE` requires the `CAP_NET_RAW` capability (works when running as root)"]
fn device() {
    // Some common network interface on Linux.
    const INTERFACES: &[&str] = &["lo", "lo0", "eth0", "wlan0"];

    let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    assert_eq!(socket.device().unwrap(), None);

    for interface in INTERFACES.iter() {
        if let Err(err) = socket.bind_device(Some(interface.as_bytes())) {
            // Network interface is not available try another.
            if matches!(err.raw_os_error(), Some(libc::ENODEV)) {
                eprintln!("error binding to device (`{interface}`): {err}");
                continue;
            } else {
                panic!("unexpected error binding device: {err}");
            }
        }
        assert_eq!(
            socket.device().unwrap().as_deref(),
            Some(interface.as_bytes())
        );

        socket.bind_device(None).unwrap();
        assert_eq!(socket.device().unwrap(), None);
        // Just need to do it with one interface.
        return;
    }

    panic!("failed to bind to any device.");
}

#[cfg(all(
    feature = "all",
    any(
        target_os = "ios",
        target_os = "visionos",
        target_os = "macos",
        target_os = "tvos",
        target_os = "watchos",
        target_os = "solaris",
        target_os = "illumos",
    )
))]
#[test]
fn device() {
    // Some common network interface on macOS.
    const INTERFACES: &[&str] = &["lo\0", "lo0\0", "en0\0"];

    let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    assert_eq!(socket.device_index_v4().unwrap(), None);

    for interface in INTERFACES.iter() {
        let iface_index = std::num::NonZeroU32::new(unsafe {
            libc::if_nametoindex(interface.as_ptr() as *const _)
        });
        // If no index is returned, try another interface alias
        if iface_index.is_none() {
            continue;
        }
        if let Err(err) = socket.bind_device_by_index_v4(iface_index) {
            // Network interface is not available try another.
            if matches!(err.raw_os_error(), Some(libc::ENODEV)) {
                eprintln!("error binding to device (`{interface}`): {err}");
                continue;
            } else {
                panic!("unexpected error binding device: {err}");
            }
        }
        assert_eq!(socket.device_index_v4().unwrap(), iface_index);

        socket.bind_device_by_index_v4(None).unwrap();
        assert_eq!(socket.device_index_v4().unwrap(), None);
        // Just need to do it with one interface.
        return;
    }

    panic!("failed to bind to any device.");
}

#[cfg(all(
    feature = "all",
    any(
        target_os = "ios",
        target_os = "visionos",
        target_os = "macos",
        target_os = "tvos",
        target_os = "watchos",
        target_os = "solaris",
        target_os = "illumos",
    )
))]
#[test]
fn device_v6() {
    // Some common network interface on macOS.
    const INTERFACES: &[&str] = &["lo\0", "lo0\0", "en0\0"];

    let socket = Socket::new(Domain::IPV6, Type::STREAM, None).unwrap();
    assert_eq!(socket.device_index_v6().unwrap(), None);

    for interface in INTERFACES.iter() {
        let iface_index = std::num::NonZeroU32::new(unsafe {
            libc::if_nametoindex(interface.as_ptr() as *const _)
        });
        // If no index is returned, try another interface alias
        if iface_index.is_none() {
            continue;
        }
        if let Err(err) = socket.bind_device_by_index_v6(iface_index) {
            // Network interface is not available try another.
            if matches!(err.raw_os_error(), Some(libc::ENODEV)) {
                eprintln!("error binding to device (`{interface}`): {err}");
                continue;
            } else {
                panic!("unexpected error binding device: {err}");
            }
        }
        assert_eq!(socket.device_index_v6().unwrap(), iface_index);

        socket.bind_device_by_index_v6(None).unwrap();
        assert_eq!(socket.device_index_v6().unwrap(), None);
        // Just need to do it with one interface.
        return;
    }

    panic!("failed to bind to any device.");
}

#[cfg(all(
    feature = "all",
    any(
        target_os = "android",
        target_os = "freebsd",
        target_os = "ios",
        target_os = "visionos",
        target_os = "linux",
        target_os = "macos",
        target_os = "tvos",
        target_os = "watchos",
    )
))]
#[test]
fn sendfile() {
    #[derive(Debug)]
    struct TestFile {
        path: &'static str,
        data: &'static [u8],
    }

    const HELLO_WORLD: TestFile = TestFile {
        path: "tests/data/hello_world.txt",
        data: include_bytes!("data/hello_world.txt"),
    };

    const LOREM: TestFile = TestFile {
        path: "tests/data/lorem_ipsum.txt",
        data: include_bytes!("data/lorem_ipsum.txt"),
    };

    let listener = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP)).unwrap();
    listener.bind(&any_ipv4()).unwrap();
    listener.listen(1).unwrap();
    let address = listener.local_addr().unwrap();

    let sender = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP)).unwrap();
    sender.connect(&address).unwrap();

    let (receiver, _) = listener.accept().unwrap();

    // Send a simple hello world file.
    {
        let file = File::open(HELLO_WORLD.path).unwrap();
        let n = sender.sendfile(&file, 0, None).unwrap();
        assert_eq!(n, HELLO_WORLD.data.len());

        let mut buf = Vec::with_capacity(HELLO_WORLD.data.len() + 1);
        let n = receiver.recv(buf.spare_capacity_mut()).unwrap();
        assert_eq!(n, HELLO_WORLD.data.len());
        unsafe { buf.set_len(n) };
        assert_eq!(buf, HELLO_WORLD.data);
    }

    // Send a larger file in two calls.
    {
        let file = File::open(LOREM.path).unwrap();
        let n = sender
            .sendfile(&file, 0, NonZeroUsize::new(LOREM.data.len() / 2))
            .unwrap();
        assert_eq!(n, LOREM.data.len() / 2);

        let offset = n;
        let n = sender.sendfile(&file, offset, None).unwrap();
        assert_eq!(offset + n, LOREM.data.len());

        let mut buf = Vec::with_capacity(LOREM.data.len() + 1);
        let mut total = 0;
        while total < LOREM.data.len() {
            let n = receiver.recv(buf.spare_capacity_mut()).unwrap();
            unsafe { buf.set_len(buf.len() + n) };
            total += n;
        }
        assert_eq!(total, LOREM.data.len());
        assert_eq!(buf, LOREM.data);
    }
}

#[cfg(all(
    feature = "all",
    any(
        target_os = "android",
        target_os = "freebsd",
        target_os = "fuchsia",
        target_os = "linux",
    )
))]
#[test]
fn is_listener() {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP)).unwrap();
    assert_eq!(socket.is_listener().unwrap(), false);

    socket.listen(1).unwrap();
    assert_eq!(socket.is_listener().unwrap(), true);
}

#[cfg(all(
    feature = "all",
    any(
        target_os = "android",
        // TODO: add FreeBSD.
        // target_os = "freebsd",
        target_os = "fuchsia",
        target_os = "linux",
    )
))]
#[test]
fn domain() {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    assert_eq!(socket.domain().unwrap(), Domain::IPV4);

    let socket = Socket::new(Domain::IPV6, Type::STREAM, None).unwrap();
    assert_eq!(socket.domain().unwrap(), Domain::IPV6);

    let socket = Socket::new(Domain::UNIX, Type::STREAM, None).unwrap();
    assert_eq!(socket.domain().unwrap(), Domain::UNIX);
}

#[cfg(all(
    feature = "all",
    any(
        target_os = "android",
        target_os = "freebsd",
        target_os = "fuchsia",
        target_os = "linux",
        target_os = "windows",
    )
))]
#[test]
fn protocol() {
    let socket = Socket::new(Domain::UNIX, Type::STREAM, None).unwrap();
    assert_eq!(socket.protocol().unwrap(), None);

    /* Don't have permission for this on CI.
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::ICMPV4)).unwrap();
    assert_eq!(socket.protocol().unwrap(), Some(Protocol::ICMPV4));

    let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::ICMPV6)).unwrap();
    assert_eq!(socket.protocol().unwrap(), Some(Protocol::ICMPV6));
    */

    let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP)).unwrap();
    assert_eq!(socket.protocol().unwrap(), Some(Protocol::TCP));

    let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP)).unwrap();
    assert_eq!(socket.protocol().unwrap(), Some(Protocol::UDP));
}

#[test]
fn r#type() {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    assert_eq!(socket.r#type().unwrap(), Type::STREAM);

    #[cfg(not(target_os = "vita"))]
    {
        let socket = Socket::new(Domain::IPV6, Type::DGRAM, None).unwrap();
        assert_eq!(socket.r#type().unwrap(), Type::DGRAM);
    }

    // macos doesn't support seqpacket
    #[cfg(all(
        unix,
        not(any(
            target_os = "ios",
            target_os = "visionos",
            target_os = "macos",
            target_os = "tvos",
            target_os = "watchos",
            target_os = "vita",
            target_os = "cygwin",
        )),
        feature = "all",
    ))]
    {
        let socket = Socket::new(Domain::UNIX, Type::SEQPACKET, None).unwrap();
        assert_eq!(socket.r#type().unwrap(), Type::SEQPACKET);
    }
}

#[cfg(all(feature = "all", target_os = "linux"))]
#[test]
fn cpu_affinity() {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();

    // NOTE: This requires at least 2 CPU cores.
    let cpu = socket.cpu_affinity().unwrap();
    let want = usize::from(cpu == 0);

    socket.set_cpu_affinity(want).unwrap();
    assert_eq!(socket.cpu_affinity().unwrap(), want);
}

#[test]
fn niche() {
    if mem::size_of::<Option<TcpStream>>() == mem::size_of::<TcpStream>() {
        assert_eq!(mem::size_of::<Option<Socket>>(), mem::size_of::<Socket>());
    }
}

fn any_ipv4() -> SockAddr {
    SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0).into()
}

/// Assume the `buf`fer to be initialised.
// TODO: replace with `MaybeUninit::slice_assume_init_ref` once stable.
#[cfg(not(target_os = "vita"))] // Loopback has special behavior on vita
unsafe fn assume_init(buf: &[MaybeUninit<u8>]) -> &[u8] {
    &*(buf as *const [MaybeUninit<u8>] as *const [u8])
}

/// Macro to create a simple test to set and get a socket option.
macro_rules! test {
    // Test using the `arg`ument as expected return value.
    ($( #[ $attr: meta ] )* $get_fn: ident, $set_fn: ident ( $arg: expr ) ) => {
        test!($( #[$attr] )* $get_fn, $set_fn($arg), $arg);
    };
    ($( #[ $attr: meta ] )* $get_fn: ident, $set_fn: ident ( $arg: expr ), $expected: expr ) => {
        #[test]
        $( #[$attr] )*
        fn $get_fn() {
            test!(__ Domain::IPV4, $get_fn, $set_fn($arg), $expected);
            #[cfg(not(target_os = "vita"))]
            test!(__ Domain::IPV6, $get_fn, $set_fn($arg), $expected);
        }
    };
    // Only test using a IPv4 socket.
    (IPv4 $get_fn: ident, $set_fn: ident ( $arg: expr ) ) => {
        #[test]
        fn $get_fn() {
            test!(__ Domain::IPV4, $get_fn, $set_fn($arg), $arg);
        }
    };
    // Only test using a IPv6 socket.
    (IPv6 $get_fn: ident, $set_fn: ident ( $arg: expr ) ) => {
        #[test]
        fn $get_fn() {
            test!(__ Domain::IPV6, $get_fn, $set_fn($arg), $arg);
        }
    };

    // Internal to this macro.
    (__ $ty: expr, $get_fn: ident, $set_fn: ident ( $arg: expr ), $expected: expr ) => {
        let socket = Socket::new($ty, Type::STREAM, None).expect("failed to create `Socket`");

        let initial = socket.$get_fn().expect("failed to get initial value");
        let arg = $arg;
        assert_ne!(initial, arg, "initial value and argument are the same");

        socket.$set_fn(arg).expect("failed to set option");
        let got = socket.$get_fn().expect("failed to get value");
        let expected = $expected;
        assert_eq!(got, expected, "set and get values differ");
    };
}

const SET_BUF_SIZE: usize = 4096;
// Linux doubles the buffer size for kernel usage, and exposes that when
// retrieving the buffer size.
#[cfg(not(target_os = "linux"))]
const GET_BUF_SIZE: usize = SET_BUF_SIZE;
#[cfg(target_os = "linux")]
const GET_BUF_SIZE: usize = 2 * SET_BUF_SIZE;

test!(tcp_nodelay, set_tcp_nodelay(true));
test!(
    recv_buffer_size,
    set_recv_buffer_size(SET_BUF_SIZE),
    GET_BUF_SIZE
);
test!(
    send_buffer_size,
    set_send_buffer_size(SET_BUF_SIZE),
    GET_BUF_SIZE
);
#[cfg(not(target_os = "redox"))]
test!(out_of_band_inline, set_out_of_band_inline(true));
test!(reuse_address, set_reuse_address(true));
#[cfg(all(
    feature = "all",
    not(any(
        windows,
        target_os = "solaris",
        target_os = "illumos",
        target_os = "cygwin",
    ))
))]
test!(reuse_port, set_reuse_port(true));
#[cfg(all(feature = "all", target_os = "freebsd"))]
test!(reuse_port_lb, set_reuse_port_lb(true));
#[cfg(all(
    feature = "all",
    unix,
    not(any(target_os = "redox", target_os = "cygwin")),
))]
test!(
    #[cfg_attr(target_os = "linux", ignore = "Different value returned")]
    tcp_mss,
    set_tcp_mss(256)
);
#[cfg(all(feature = "all", target_os = "linux"))]
test!(
    #[ignore = "setting `IP_TRANSPARENT` requires the `CAP_NET_ADMIN` capability (works when running as root)"]
    ip_transparent_v4,
    set_ip_transparent_v4(true)
);
#[cfg(all(feature = "all", any(target_os = "fuchsia", target_os = "linux")))]
test!(
    #[ignore = "setting `SO_MARK` requires the `CAP_NET_ADMIN` capability (works when running as root)"]
    mark,
    set_mark(123)
);
#[cfg(all(
    feature = "all",
    any(target_os = "android", target_os = "fuchsia", target_os = "linux")
))]
test!(tcp_cork, set_tcp_cork(true));
#[cfg(all(
    feature = "all",
    any(target_os = "android", target_os = "fuchsia", target_os = "linux")
))]
test!(tcp_quickack, set_tcp_quickack(false));
#[cfg(all(
    feature = "all",
    any(target_os = "android", target_os = "fuchsia", target_os = "linux")
))]
test!(tcp_thin_linear_timeouts, set_tcp_thin_linear_timeouts(true));
test!(linger, set_linger(Some(Duration::from_secs(10))));
test!(
    read_timeout,
    set_read_timeout(Some(Duration::from_secs(10)))
);
test!(keepalive, set_keepalive(true));
#[cfg(all(feature = "all", any(target_os = "fuchsia", target_os = "linux")))]
test!(freebind_v4, set_freebind_v4(true));
#[cfg(all(feature = "all", target_os = "linux"))]
test!(IPv6 freebind_v6, set_freebind_v6(true));

test!(IPv4 ttl_v4, set_ttl_v4(40));

#[cfg(not(any(
    target_os = "fuchsia",
    target_os = "redox",
    target_os = "solaris",
    target_os = "illumos",
    target_os = "haiku",
    target_os = "cygwin",
)))]
test!(IPv4 tos_v4, set_tos_v4(96));

#[cfg(not(any(
    target_os = "dragonfly",
    target_os = "fuchsia",
    target_os = "hurd",
    target_os = "illumos",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "redox",
    target_os = "solaris",
    target_os = "windows",
    target_os = "vita",
    target_os = "haiku",
    target_os = "cygwin",
)))]
test!(IPv4 recv_tos_v4, set_recv_tos_v4(true));

#[cfg(not(any(windows, target_os = "cygwin")))] // TODO: returns `WSAENOPROTOOPT` (10042) on Windows.
test!(IPv4 broadcast, set_broadcast(true));

#[cfg(not(target_os = "vita"))]
test!(IPv6 unicast_hops_v6, set_unicast_hops_v6(20));

#[cfg(not(any(
    windows,
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "vita",
    target_os = "cygwin",
)))]
test!(IPv6 only_v6, set_only_v6(true));
// IPv6 socket are already IPv6 only on FreeBSD and Windows.
#[cfg(any(windows, target_os = "freebsd", target_os = "cygwin"))]
test!(IPv6 only_v6, set_only_v6(false));

#[cfg(all(
    feature = "all",
    any(
        target_os = "android",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "fuchsia",
        target_os = "linux",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd"
    )
))]
test!(IPv6 tclass_v6, set_tclass_v6(96));

#[cfg(not(any(
    target_os = "dragonfly",
    target_os = "fuchsia",
    target_os = "hurd",
    target_os = "illumos",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "redox",
    target_os = "solaris",
    target_os = "windows",
    target_os = "vita",
    target_os = "haiku",
    target_os = "cygwin",
)))]
test!(IPv6 recv_tclass_v6, set_recv_tclass_v6(true));

#[cfg(all(
    feature = "all",
    not(any(
        target_os = "dragonfly",
        target_os = "fuchsia",
        target_os = "hurd",
        target_os = "illumos",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "redox",
        target_os = "solaris",
        target_os = "windows",
        target_os = "vita",
        target_os = "haiku",
        target_os = "cygwin",
    ))
))]
test!(IPv6 recv_hoplimit_v6, set_recv_hoplimit_v6(true));

#[cfg(all(
    feature = "all",
    any(target_os = "android", target_os = "fuchsia", target_os = "linux")
))]
test!(
    tcp_user_timeout,
    set_tcp_user_timeout(Some(Duration::from_secs(10)))
);

#[cfg(all(feature = "all", target_os = "linux"))]
test!(IPv4 multicast_all_v4, set_multicast_all_v4(false));
#[cfg(all(feature = "all", target_os = "linux"))]
test!(IPv6 multicast_all_v6, set_multicast_all_v6(false));

#[test]
#[cfg(not(any(
    target_os = "haiku",
    target_os = "illumos",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "redox",
    target_os = "solaris",
    target_os = "vita",
    target_os = "cygwin",
)))]
fn join_leave_multicast_v4_n() {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, None).unwrap();
    let multiaddr = Ipv4Addr::new(224, 0, 1, 1);
    let interface = socket2::InterfaceIndexOrAddress::Index(0);
    match socket.leave_multicast_v4_n(&multiaddr, &interface) {
        Ok(()) => panic!("leaving an unjoined group should fail"),
        Err(err) => {
            assert_eq!(err.kind(), io::ErrorKind::AddrNotAvailable);
            #[cfg(unix)]
            assert_eq!(err.raw_os_error(), Some(libc::EADDRNOTAVAIL));
        }
    };
    socket
        .join_multicast_v4_n(&multiaddr, &interface)
        .expect("join multicast group");
    socket
        .leave_multicast_v4_n(&multiaddr, &interface)
        .expect("leave multicast group");
}

#[test]
#[cfg(not(any(
    target_os = "dragonfly",
    target_os = "haiku",
    target_os = "hurd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "redox",
    target_os = "fuchsia",
    target_os = "vita",
)))]
fn join_leave_ssm_v4() {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, None).unwrap();
    let g = Ipv4Addr::new(232, 123, 52, 36);
    let s = Ipv4Addr::new(62, 40, 109, 31);
    let interface = Ipv4Addr::new(0, 0, 0, 0);
    socket.join_ssm_v4(&s, &g, &interface).expect("Joined SSM");
    socket.leave_ssm_v4(&s, &g, &interface).expect("Left SSM");
}

#[test]
#[cfg(all(feature = "all", not(target_os = "redox")))]
fn header_included() {
    let socket = match Socket::new(Domain::IPV4, Type::RAW, None) {
        Ok(socket) => socket,
        // Need certain permissions to create a raw sockets.
        Err(ref err) if err.kind() == io::ErrorKind::PermissionDenied => return,
        #[cfg(unix)]
        Err(ref err) if err.raw_os_error() == Some(libc::EPROTONOSUPPORT) => return,
        Err(err) => panic!("unexpected error creating socket: {err}"),
    };

    let initial = socket
        .header_included_v4()
        .expect("failed to get initial value");
    assert_eq!(initial, false, "initial value and argument are the same");

    socket
        .set_header_included_v4(true)
        .expect("failed to set option");
    let got = socket.header_included_v4().expect("failed to get value");
    assert_eq!(got, true, "set and get values differ");
}

#[test]
#[cfg(all(
    feature = "all",
    not(any(
        target_os = "redox",
        target_os = "espidf",
        target_os = "openbsd",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd"
    ))
))]
fn header_included_ipv6() {
    let socket = match Socket::new(Domain::IPV6, Type::RAW, None) {
        Ok(socket) => socket,
        // Need certain permissions to create a raw sockets.
        Err(ref err) if err.kind() == io::ErrorKind::PermissionDenied => return,
        #[cfg(unix)]
        Err(ref err) if err.raw_os_error() == Some(libc::EPROTONOSUPPORT) => return,
        Err(err) => panic!("unexpected error creating socket: {err}"),
    };

    let initial = socket
        .header_included_v6()
        .expect("failed to get initial value");
    assert_eq!(initial, false, "initial value and argument are the same");

    socket
        .set_header_included_v6(true)
        .expect("failed to set option");
    let got = socket.header_included_v6().expect("failed to get value");
    assert_eq!(got, true, "set and get values differ");
}

#[test]
#[cfg(all(
    feature = "all",
    any(
        target_os = "android",
        target_os = "fuchsia",
        target_os = "linux",
        target_os = "windows"
    )
))]
fn original_dst_v4() {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    #[cfg(not(target_os = "windows"))]
    let expected = Some(libc::ENOENT);
    #[cfg(target_os = "windows")]
    let expected = Some(windows_sys::Win32::Networking::WinSock::WSAEINVAL);

    match socket.original_dst_v4() {
        Ok(_) => panic!("original_dst_v4 on non-redirected socket should fail"),
        Err(err) => assert_eq!(err.raw_os_error(), expected),
    }

    let socket = Socket::new(Domain::IPV6, Type::STREAM, None).unwrap();
    match socket.original_dst_v4() {
        Ok(_) => panic!("original_dst_v4 on non-redirected socket should fail"),
        Err(err) => assert_eq!(err.raw_os_error(), expected),
    }
}

#[test]
#[cfg(all(
    feature = "all",
    any(target_os = "android", target_os = "fuchsia", target_os = "linux")
))]
fn original_dst_v6() {
    let socket = Socket::new(Domain::IPV6, Type::STREAM, None).unwrap();
    #[cfg(not(target_os = "windows"))]
    let expected = Some(libc::ENOENT);
    #[cfg(target_os = "windows")]
    let expected = Some(windows_sys::Win32::Networking::WinSock::WSAEINVAL);
    #[cfg(not(target_os = "windows"))]
    let expected_v4 = Some(libc::EOPNOTSUPP);
    #[cfg(target_os = "windows")]
    let expected_v4 = Some(windows_sys::Win32::Networking::WinSock::WSAEINVAL);
    match socket.original_dst_v6() {
        Ok(_) => panic!("original_dst_v6 on non-redirected socket should fail"),
        Err(err) => assert_eq!(err.raw_os_error(), expected),
    }

    // Not supported on IPv4 socket.
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    match socket.original_dst_v6() {
        Ok(_) => panic!("original_dst_v6 on non-redirected socket should fail"),
        Err(err) => assert_eq!(err.raw_os_error(), expected_v4),
    }
}

#[test]
#[cfg(all(feature = "all", any(target_os = "freebsd", target_os = "linux")))]
fn tcp_congestion() {
    let socket: Socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    // Get and set current tcp_ca
    let origin_tcp_ca = socket
        .tcp_congestion()
        .expect("failed to get tcp congestion algorithm");
    socket
        .set_tcp_congestion(&origin_tcp_ca)
        .expect("failed to set tcp congestion algorithm");
    // Return a Err when set a non-exist tcp_ca
    socket
        .set_tcp_congestion(b"tcp_congestion_does_not_exist")
        .unwrap_err();
    let cur_tcp_ca = socket.tcp_congestion().unwrap();
    assert_eq!(
        cur_tcp_ca, origin_tcp_ca,
        "expected {origin_tcp_ca:?} but get {cur_tcp_ca:?}"
    );
    let cur_tcp_ca = cur_tcp_ca.splitn(2, |num| *num == 0).next().unwrap();
    const OPTIONS: [&[u8]; 2] = [
        b"cubic",
        #[cfg(target_os = "linux")]
        b"reno",
        #[cfg(target_os = "freebsd")]
        b"newreno",
    ];
    // Set a new tcp ca
    #[cfg(target_os = "linux")]
    let new_tcp_ca = if cur_tcp_ca == OPTIONS[0] {
        OPTIONS[1]
    } else {
        OPTIONS[0]
    };
    #[cfg(target_os = "freebsd")]
    let new_tcp_ca = OPTIONS[1];
    socket.set_tcp_congestion(new_tcp_ca).unwrap();
    // Check if new tcp ca is successfully set
    let cur_tcp_ca = socket.tcp_congestion().unwrap();
    assert_eq!(
        cur_tcp_ca.splitn(2, |num| *num == 0).next().unwrap(),
        new_tcp_ca,
    );
}

#[test]
#[ignore = "DCCP support is not enabled in all kernels of majors Linux distros"]
#[cfg(all(feature = "all", target_os = "linux"))]
fn dccp() {
    let listener = Socket::new(Domain::IPV4, Type::DCCP, Some(Protocol::DCCP)).unwrap();
    let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap().into();
    listener.set_dccp_service(45).unwrap();
    assert!(listener.dccp_service().unwrap() == 45);
    assert!(listener.dccp_cur_mps().unwrap() > 0);
    assert!(listener.dccp_available_ccids::<4>().unwrap().len() >= 3);
    assert!(
        listener.dccp_send_cscov().unwrap() == 0,
        "sender cscov should be zero by default"
    );
    listener.set_dccp_ccid(2).unwrap();
    listener.set_dccp_qpolicy_txqlen(6).unwrap();
    assert!(listener.dccp_qpolicy_txqlen().unwrap() == 6);
    listener.bind(&addr).unwrap();
    listener.listen(10).unwrap();

    let mut client = Socket::new(Domain::IPV4, Type::DCCP, Some(Protocol::DCCP)).unwrap();
    client.set_dccp_service(45).unwrap();
    client.connect(&addr).unwrap();

    let (mut accepted, _) = listener.accept().unwrap();
    let msg = "Hello World!";
    assert!(client.write(msg.as_bytes()).unwrap() == msg.len());
    let mut recv_buf = [0_u8; 64];
    assert!(accepted.read(&mut recv_buf).unwrap() == msg.len());
}

#[test]
#[cfg(all(feature = "all", target_os = "linux"))]
fn cookie() {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    let first_socket_cookie = socket.cookie();
    match first_socket_cookie {
        Ok(_) => {}
        Err(err) => panic!("Could not get socket cookie, err: {err}"),
    }

    //Fetch cookie again and make sure it's the same value (ALWAYS should be, smoke test)
    let second_socket_cookie = socket.cookie();
    match second_socket_cookie {
        Ok(cookie) => assert_eq!(cookie, first_socket_cookie.unwrap()),
        Err(err) => panic!("Could not get socket cookie a second time, err: {err}"),
    }
}

#[cfg(all(unix, target_os = "linux"))]
#[test]
fn set_passcred() {
    let socket = Socket::new(Domain::UNIX, Type::DGRAM, None).unwrap();
    assert!(!socket.passcred().unwrap());

    socket.set_passcred(true).unwrap();
    assert!(socket.passcred().unwrap());

    let socket = Socket::new(Domain::UNIX, Type::STREAM, None).unwrap();
    assert!(!socket.passcred().unwrap());

    socket.set_passcred(true).unwrap();
    assert!(socket.passcred().unwrap());
}

#[cfg(all(feature = "all", target_os = "linux"))]
#[test]
fn set_priority() {
    let socket = Socket::new(Domain::UNIX, Type::DGRAM, None).unwrap();
    assert!(socket.priority().unwrap() == 0);

    // test priorities 6 .. 0; values above 6 require additional priviledges
    for i in (0..=6).rev() {
        socket.set_priority(i).unwrap();
        assert!(socket.priority().unwrap() == i);
    }
}

#[cfg(all(feature = "all", target_os = "linux"))]
#[test]
fn set_busy_poll() {
    let socket = Socket::new(Domain::UNIX, Type::DGRAM, None).unwrap();
    assert!(socket.busy_poll().unwrap() == 0);

    // test busy poll values 0 .. 6; values above 6 require additional priviledges
    for i in (0..=6).rev() {
        socket.set_busy_poll(i).unwrap();
        assert!(socket.busy_poll().unwrap() == i);
    }
}

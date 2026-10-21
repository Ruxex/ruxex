use ruxex_app_server_mock::MockAppServer;
use ruxex_app_server_mock::MockScenario;
use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::net::TcpListener;

#[derive(Default)]
struct Options {
    help: bool,
    scenario: Option<PathBuf>,
    ruxex_home: Option<PathBuf>,
    listen: Option<SocketAddr>,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let options = parse_options()?;
    if options.help {
        eprintln!(
            "Usage: ruxex-app-server-mock [--scenario PATH] [--ruxex-home PATH] [--listen ADDR]"
        );
        return Ok(());
    }

    let mut builder = MockAppServer::builder();
    if let Some(path) = options.scenario {
        builder = builder.with_scenario(MockScenario::from_path(path)?);
    }
    if let Some(path) = options.ruxex_home {
        builder = builder.with_ruxex_home(path);
    }
    let server = builder.build()?;

    match options.listen {
        Some(address) => {
            server
                .run_websocket(TcpListener::bind(address).await?)
                .await
        }
        None => server.run_stdio().await,
    }
}

fn parse_options() -> io::Result<Options> {
    let mut options = Options {
        scenario: std::env::var_os("RUXEX_MOCK_SCENARIO").map(PathBuf::from),
        ruxex_home: std::env::var_os("RUXEX_MOCK_HOME")
            .or_else(|| std::env::var_os("RUXEX_HOME"))
            .map(PathBuf::from),
        listen: std::env::var_os("RUXEX_MOCK_LISTEN")
            .map(|value| parse_address(&value.to_string_lossy()))
            .transpose()?,
        ..Options::default()
    };
    let mut args = std::env::args_os().skip(1);
    while let Some(argument) = args.next() {
        match argument.to_string_lossy().as_ref() {
            "--help" | "-h" => options.help = true,
            "--scenario" => options.scenario = Some(next_path(&mut args, "--scenario")?),
            "--ruxex-home" => options.ruxex_home = Some(next_path(&mut args, "--ruxex-home")?),
            "--listen" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "--listen requires an address")
                })?;
                options.listen = Some(parse_address(&value.to_string_lossy())?);
            }
            unknown => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown argument: {unknown}"),
                ));
            }
        }
    }
    Ok(options)
}

fn next_path(
    args: &mut impl Iterator<Item = std::ffi::OsString>,
    option: &str,
) -> io::Result<PathBuf> {
    args.next().map(PathBuf::from).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{option} requires a path"),
        )
    })
}

fn parse_address(value: &str) -> io::Result<SocketAddr> {
    value.parse().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid listen address {value:?}: {error}"),
        )
    })
}

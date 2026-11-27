use eframe::egui;
use rust_radio_gui::{ExternalProvider, RadioPanel, PANEL_HEIGHT, PANEL_WIDTH};
use std::io;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::process::{Command, Stdio};

mod presets_file;

struct StandaloneRadio {
    radio: RadioPanel,
    stations: presets_file::PresetFile,
    external_launch: Option<Box<dyn LaunchProcess>>,
    #[cfg(feature = "nrsp-native")]
    presets: presets_file::PresetFile,
}

impl eframe::App for StandaloneRadio {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::BLACK))
            .show(ctx, |ui| self.radio.show(ui));
        self.stations.poll(&mut self.radio);
        if dispatch_external_provider(
            self.radio.take_external_provider_intent(),
            &SystemUrlLauncher,
            &mut self.external_launch,
        ) == LaunchStatus::Failed
        {
            self.radio.external_provider_launch_failed();
        }
        #[cfg(feature = "nrsp-native")]
        self.presets.poll(&mut self.radio);
    }
}

trait UrlLauncher {
    fn open(&self, provider: ExternalProvider) -> io::Result<Box<dyn LaunchProcess>>;
}

trait LaunchProcess {
    /// Poll without waiting for the system URL handler to exit.
    fn try_wait_success(&mut self) -> io::Result<Option<bool>>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LaunchStatus {
    Idle,
    Pending,
    Dispatched,
    Failed,
}

impl LaunchProcess for std::process::Child {
    fn try_wait_success(&mut self) -> io::Result<Option<bool>> {
        self.try_wait()
            .map(|status| status.map(|status| status.success()))
    }
}

struct SystemUrlLauncher;

impl UrlLauncher for SystemUrlLauncher {
    fn open(&self, provider: ExternalProvider) -> io::Result<Box<dyn LaunchProcess>> {
        let url = provider.official_url();
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            #[cfg(target_os = "macos")]
            let mut command = Command::new("open");
            #[cfg(target_os = "linux")]
            let mut command = Command::new("xdg-open");
            command.arg(url);

            let child = command
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            Ok(Box::new(child))
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = provider;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "No system URL handler is configured",
            ))
        }
    }
}

fn dispatch_external_provider<L: UrlLauncher>(
    intent: Option<ExternalProvider>,
    launcher: &L,
    launch_process: &mut Option<Box<dyn LaunchProcess>>,
) -> LaunchStatus {
    if let Some(provider) = intent {
        match launcher.open(provider) {
            Ok(process) => *launch_process = Some(process),
            Err(_) => return LaunchStatus::Failed,
        }
    }
    let result = launch_process
        .as_mut()
        .map(|process| process.try_wait_success());
    match result {
        Some(Ok(Some(true))) => {
            *launch_process = None;
            LaunchStatus::Dispatched
        }
        Some(Ok(Some(false))) | Some(Err(_)) => {
            *launch_process = None;
            LaunchStatus::Failed
        }
        Some(Ok(None)) => LaunchStatus::Pending,
        None => LaunchStatus::Idle,
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("RUST RADIO / MICRO")
            .with_inner_size([PANEL_WIDTH, PANEL_HEIGHT])
            .with_min_inner_size([PANEL_WIDTH, PANEL_HEIGHT]),
        ..Default::default()
    };
    eframe::run_native(
        "Rust Radio",
        options,
        Box::new(|_cc| {
            let mut radio = RadioPanel::new_standalone();
            radio.enable_input_discovery();
            let stations = presets_file::PresetFile::load_ip(&mut radio);
            #[cfg(feature = "nrsp-native")]
            let presets = presets_file::PresetFile::load(&mut radio);
            Ok(Box::new(StandaloneRadio {
                radio,
                stations,
                external_launch: None,
                #[cfg(feature = "nrsp-native")]
                presets,
            }))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MockLauncher {
        urls: Mutex<Vec<&'static str>>,
        fail_spawn: bool,
        process_success: Option<bool>,
    }

    struct MockProcess(Option<bool>);

    impl LaunchProcess for MockProcess {
        fn try_wait_success(&mut self) -> io::Result<Option<bool>> {
            Ok(self.0)
        }
    }

    impl UrlLauncher for MockLauncher {
        fn open(&self, provider: ExternalProvider) -> io::Result<Box<dyn LaunchProcess>> {
            self.urls.lock().unwrap().push(provider.official_url());
            if self.fail_spawn {
                Err(io::Error::other("mock launch failure"))
            } else {
                Ok(Box::new(MockProcess(self.process_success)))
            }
        }
    }

    #[test]
    fn dispatch_sends_only_the_fixed_siriusxm_url_once() {
        let launcher = MockLauncher::default();
        let mut process = None;
        assert_eq!(
            dispatch_external_provider(Some(ExternalProvider::SiriusXm), &launcher, &mut process,),
            LaunchStatus::Pending
        );
        assert_eq!(
            dispatch_external_provider(None, &launcher, &mut process),
            LaunchStatus::Pending
        );
        assert!(process.is_some());
        assert_eq!(
            *launcher.urls.lock().unwrap(),
            ["https://player.siriusxm.com/"]
        );
    }

    #[test]
    fn successful_handler_exit_is_dispatched_without_claiming_playback() {
        let launcher = MockLauncher {
            urls: Mutex::new(Vec::new()),
            fail_spawn: false,
            process_success: Some(true),
        };
        let mut process = None;
        assert_eq!(
            dispatch_external_provider(Some(ExternalProvider::SiriusXm), &launcher, &mut process,),
            LaunchStatus::Dispatched
        );
        assert!(process.is_none());
    }

    #[test]
    fn nonzero_handler_exit_is_reported_as_failure() {
        let launcher = MockLauncher {
            urls: Mutex::new(Vec::new()),
            fail_spawn: false,
            process_success: Some(false),
        };
        let mut process = None;
        assert_eq!(
            dispatch_external_provider(Some(ExternalProvider::SiriusXm), &launcher, &mut process,),
            LaunchStatus::Failed
        );
        assert!(process.is_none());
        assert!(launcher.urls.lock().unwrap().len() == 1);
    }

    #[test]
    fn immediate_handler_spawn_failure_is_reported() {
        let launcher = MockLauncher {
            urls: Mutex::new(Vec::new()),
            fail_spawn: true,
            process_success: Some(false),
        };
        let mut process = None;
        assert_eq!(
            dispatch_external_provider(Some(ExternalProvider::SiriusXm), &launcher, &mut process,),
            LaunchStatus::Failed
        );
        assert!(process.is_none());
    }
}

use std::{collections::HashMap, io::Write, os::unix::net::UnixStream, sync::Arc, time::Duration};

use common::{DEFAULT_CONTROL_SOCK_PATH, DaemonStatus};
use iced::Length;
use iced::widget::{Column, Container, Row, Text, row};
use iced::{
    Subscription, Task,
    futures::future::join,
    task::{Never, Sipper},
};

#[derive(Debug)]
enum UiMonitorStatus {
    Managed { brightness: u16, target: u16 },
    Unmanaged,
}

/// UI state: tracks everything that needs to be displayed
#[derive(Debug, Default)]
struct State {
    lux: u32,
    monitors: HashMap<String, UiMonitorStatus>,
}

/// Messages: used to communicates updates to the UI state
#[derive(Debug, Clone)]
enum Message {
    StatusUpdate(Arc<DaemonStatus>),
}

impl State {
    fn view(&self) -> iced::Element<'_, Message> {
        let lux_text = Text::new(format!("Lux: {}", self.lux)).width(Length::Fill);

        let monitor_rows = self.monitors.iter().map(|(monitor_id, status)| {
            let status_text = match status {
                UiMonitorStatus::Managed { brightness, target } => {
                    format!(
                        "Monitor {}: Brightness: {}, Target: {}",
                        monitor_id, brightness, target
                    )
                }
                UiMonitorStatus::Unmanaged => format!("Monitor {}: Unmanaged", monitor_id),
            };

            row![Text::new(status_text)].spacing(10).width(Length::Fill)
        });

        let mut content = Column::new().spacing(10).push(lux_text).push(
            Row::new()
                .spacing(10)
                .push(Text::new("Monitors:").width(Length::Fill))
                .width(Length::Fill),
        );
        for mr in monitor_rows {
            content = content.push(mr);
        }

        let content = content.width(Length::Fill).height(Length::Fill);

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Update the state whenever the status changes
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::StatusUpdate(ds) => {
                self.lux = ds.lux;

                for ms in &ds.unmanaged_monitors {
                    if let Some(val) = self.monitors.get_mut(ms) {
                        *val = UiMonitorStatus::Unmanaged;
                    } else {
                        self.monitors.insert(ms.clone(), UiMonitorStatus::Unmanaged);
                    }
                }

                for ms in &ds.monitors {
                    let ums = UiMonitorStatus::Managed {
                        brightness: ms.brightness,
                        target: ms.target_brightness,
                    };
                    if let Some(val) = self.monitors.get_mut(&ms.display_name) {
                        *val = ums
                    } else {
                        self.monitors.insert(ms.display_name.clone(), ums);
                    }
                }
            }
        }

        Task::none()
    }
}

/// Subscribe to daemon status opdates over the socket
fn subscription() -> impl Sipper<Never, Message> {
    iced::task::sipper(async |mut output| {
        let mut s = UnixStream::connect(DEFAULT_CONTROL_SOCK_PATH).unwrap();
        loop {
            s.write_all(&[b's']).unwrap();

            let status = serde_json::Deserializer::from_reader(&s)
                .into_iter::<DaemonStatus>()
                .next()
                .unwrap()
                .unwrap();

            join(
                output.send(Message::StatusUpdate(Arc::new(status))),
                smol::Timer::after(Duration::from_secs(8)),
            )
            .await;
        }
    })
}

fn main() -> iced::Result {
    // TODO take control socket as an optional argument?

    iced::application(State::default, State::update, State::view)
        .subscription(|_| Subscription::run(subscription))
        .run()
}

use crate::config::HoardConfig;
use crate::core::HoardCmd;
use crate::gui::commands_gui::State;
use crate::gui::commands_gui::{ControlState, EditSelection};
use crate::gui::help::HELP_KEY;
use ratatui::backend::TermionBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap};
use ratatui::Terminal;
use termion::screen::AlternateScreen;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[allow(clippy::too_many_lines)]
pub fn draw(
    app_state: &mut State,
    config: &HoardConfig,
    namespace_tabs: &[&str],
    terminal: &mut Terminal<
        TermionBackend<AlternateScreen<termion::raw::RawTerminal<std::io::Stdout>>>,
    >,
) -> Result<(), eyre::Error> {
    terminal.draw(|rect| {
        let size = rect.size();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Min(2),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(size);
        let menu = namespace_tabs
            .iter()
            .map(|t| {
                Line::from(vec![Span::styled(
                    *t,
                    Style::default().fg(Color::Rgb(
                        config.primary_color.unwrap().0,
                        config.primary_color.unwrap().1,
                        config.primary_color.unwrap().2,
                    )),
                )])
            })
            .collect();

        let tabs = Tabs::new(menu)
            .select(
                app_state
                    .namespace_tab
                    .selected()
                    .expect("Always a namespace selected"),
            )
            .block(
                Block::default()
                    .title(" Hoard Namespace ")
                    .borders(Borders::ALL),
            )
            .style(Style::default().fg(Color::Rgb(
                config.primary_color.unwrap().0,
                config.primary_color.unwrap().1,
                config.primary_color.unwrap().2,
            )))
            .highlight_style(
                Style::default()
                    .fg(Color::Rgb(
                        config.secondary_color.unwrap().0,
                        config.secondary_color.unwrap().1,
                        config.secondary_color.unwrap().2,
                    ))
                    .add_modifier(Modifier::UNDERLINED),
            )
            .divider(Span::raw("|"));

        rect.render_widget(tabs, chunks[0]);

        let commands_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
            .split(chunks[1]);
        let command_detail_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Percentage(60),
                    Constraint::Length(3),
                ]
                .as_ref(),
            )
            .split(commands_chunks[1]);
        let (commands, command, tags_widget, description, input) =
            render_commands(&app_state.commands.clone(), app_state, config);
        rect.render_stateful_widget(
            commands,
            commands_chunks[0],
            &mut app_state.command_list,
        );
        rect.render_widget(tags_widget, command_detail_chunks[0]);
        rect.render_widget(description, command_detail_chunks[1]);
        rect.render_widget(command, command_detail_chunks[2]);
        rect.render_widget(input, chunks[2]);

        let (footer_left, footer_right) = get_footer_constraints(&app_state.control);
        let footer_chunk = Layout::default()
            .direction(Direction::Horizontal)
            .margin(0)
            .constraints([
                Constraint::Percentage(footer_left),
                Constraint::Percentage(footer_right),
            ])
            .split(chunks[3]);

        let control_str = &app_state.control;
        let help_hint_l = Paragraph::new(format!("{control_str}"))
            .style(Style::default().fg(Color::Rgb(
                config.primary_color.unwrap().0,
                config.primary_color.unwrap().1,
                config.primary_color.unwrap().2,
            )))
            .alignment(Alignment::Left);
        let help_hint = Paragraph::new(format!(
            "Create <Ctrl-W> | Delete <Ctrl-X> | GPT <Ctrl-A> | Help {HELP_KEY}"
        ))
        .style(Style::default().fg(Color::Rgb(
            config.primary_color.unwrap().0,
            config.primary_color.unwrap().1,
            config.primary_color.unwrap().2,
        )))
        .alignment(Alignment::Right);

        rect.render_widget(help_hint_l, footer_chunk[0]);
        if app_state.control == ControlState::Search {
            rect.render_widget(help_hint, footer_chunk[1]);
        }

        if app_state.query_gpt {
            let msg = if app_state.openai_key_set {
                State::get_default_popupmsg()
            } else {
                State::get_no_api_key_popupmsg()
            };
            let description = Paragraph::new(msg)
                .style(Style::default().fg(Color::Rgb(
                    config.primary_color.unwrap().0,
                    config.primary_color.unwrap().1,
                    config.primary_color.unwrap().2,
                )))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(get_color(
                            app_state,
                            config,
                            &EditSelection::Description,
                        )))
                        .title("GPT")
                        .border_type(BorderType::Plain),
                );
            let area = centered_rect(50, 10, size);
            rect.render_widget(Clear, area); //this clears out the background
            rect.render_widget(description, area);
        }
    })?;
    Ok(())
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

fn get_color(
    app: &State,
    config: &HoardConfig,
    command_render: &EditSelection,
) -> ratatui::style::Color {
    let highlighted = Color::Rgb(
        config.secondary_color.unwrap().0,
        config.secondary_color.unwrap().1,
        config.secondary_color.unwrap().2,
    );
    let normal = Color::Rgb(
        config.primary_color.unwrap().0,
        config.primary_color.unwrap().1,
        config.primary_color.unwrap().2,
    );
    match app.control {
        ControlState::Search | ControlState::Gpt | ControlState::KeyNotSet => normal,
        ControlState::Edit => {
            if command_render == &app.edit_selection {
                return highlighted;
            }
            normal
        }
    }
}

fn coerce_string_by_mode(s: String, app: &State, command_render: &EditSelection) -> String {
    match app.control {
        ControlState::Search | ControlState::Gpt | ControlState::KeyNotSet => s,
        ControlState::Edit => {
            if command_render == &app.edit_selection {
                return app.string_to_edit.clone();
            }
            s
        }
    }
}

#[allow(clippy::too_many_lines)]
fn render_commands<'a>(
    commands_list: &[HoardCmd],
    app: &mut State,
    config: &HoardConfig,
) -> (
    List<'a>,
    Paragraph<'a>,
    Paragraph<'a>,
    Paragraph<'a>,
    Paragraph<'a>,
) {
    let commands = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(get_color(app, config, &EditSelection::Name)))
        .title(" Commands ")
        .border_type(BorderType::Plain);

    let items: Vec<_> = commands_list
        .iter()
        .map(|command| {
            ListItem::new(Line::from(vec![Span::styled(
                command.name.clone(),
                Style::default(),
            )]))
        })
        .collect();

    let selected_command: HoardCmd = commands_list
        .get(
            app.command_list
                .selected()
                .expect("there is always a selected command"),
        )
        .get_or_insert(&HoardCmd::default())
        .clone();

    if selected_command.name.is_empty() {
        // If somehow the selection is past the last index, set it to the last element
        let new_selection = if commands_list.is_empty() {
            0
        } else {
            commands_list.len() - 1
        };
        app.command_list.select(Some(new_selection));
    }

    let list = List::new(items).block(commands).highlight_style(
        Style::default()
            .bg(Color::Rgb(
                config.secondary_color.unwrap().0,
                config.secondary_color.unwrap().1,
                config.secondary_color.unwrap().2,
            ))
            .fg(Color::Rgb(
                config.tertiary_color.unwrap().0,
                config.tertiary_color.unwrap().1,
                config.tertiary_color.unwrap().2,
            ))
            .add_modifier(Modifier::BOLD),
    );

    let command = Paragraph::new(coerce_string_by_mode(
        selected_command.command.clone(),
        app,
        &EditSelection::Command,
    ))
    .style(Style::default().fg(Color::Rgb(
        config.primary_color.unwrap().0,
        config.primary_color.unwrap().1,
        config.primary_color.unwrap().2,
    )))
    .alignment(Alignment::Left)
    .wrap(Wrap { trim: true })
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(get_color(app, config, &EditSelection::Command)))
            .title(" Hoarded command ")
            .border_type(BorderType::Plain),
    );

    let tags = Paragraph::new(coerce_string_by_mode(
        selected_command.get_tags_as_string(),
        app,
        &EditSelection::Tags,
    ))
    .style(Style::default().fg(Color::Rgb(
        config.primary_color.unwrap().0,
        config.primary_color.unwrap().1,
        config.primary_color.unwrap().2,
    )))
    .alignment(Alignment::Left)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(get_color(app, config, &EditSelection::Tags)))
            .title(" Tags ")
            .border_type(BorderType::Plain),
    );

    let description = Paragraph::new(coerce_string_by_mode(
        selected_command.description,
        app,
        &EditSelection::Description,
    ))
    .style(Style::default().fg(Color::Rgb(
        config.primary_color.unwrap().0,
        config.primary_color.unwrap().1,
        config.primary_color.unwrap().2,
    )))
    .alignment(Alignment::Left)
    .wrap(Wrap { trim: true })
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(get_color(app, config, &EditSelection::Description)))
            .title(" Description ")
            .border_type(BorderType::Plain),
    );

    let mut query_string = config.query_prefix.clone();
    query_string.push_str(&app.input.clone()[..]);
    let query_title = format!(" hoard v{VERSION} ");
    let input = Paragraph::new(query_string).block(
        Block::default()
            .style(Style::default().fg(Color::Rgb(
                config.primary_color.unwrap().0,
                config.primary_color.unwrap().1,
                config.primary_color.unwrap().2,
            )))
            .borders(Borders::ALL)
            .title(query_title),
    );

    (list, command, tags, description, input)
}

const fn get_footer_constraints(control: &ControlState) -> (u16, u16) {
    match control {
        ControlState::Search | ControlState::Gpt | ControlState::KeyNotSet => (50, 50),
        ControlState::Edit => (99, 1),
    }
}

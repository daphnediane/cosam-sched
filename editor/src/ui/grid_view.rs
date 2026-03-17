use chrono::{NaiveDate, NaiveDateTime, Timelike};
use gpui::prelude::*;
use gpui::{Context, Window, div, px, rgb, Stateful, Div};
use std::collections::{BTreeSet, HashMap};

use crate::data::{Event, PanelType, Schedule};

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct TimeSlot {
    pub key: String,
    pub start_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct GridEvent {
    pub event: Event,
    pub room_id: u32,
    pub start_slot: usize,
    pub end_slot: usize,
    pub panel_type: Option<PanelType>,
    pub color: u32,
}

#[derive(Debug, Clone)]
pub struct CollapsedTime {
    pub key: String,
    pub start_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
    pub is_collapsed: bool,
    pub label: String,
}

pub struct GridView {
    schedule: Schedule,
    selected_day: Option<NaiveDate>,
    selected_room: Option<u32>,
    time_slots: Vec<TimeSlot>,
    room_order: Vec<u32>,
    grid_events: Vec<GridEvent>,
    collapsed_times: Vec<CollapsedTime>,
}

impl GridView {
    pub fn new(
        schedule: Schedule,
        selected_day: Option<NaiveDate>,
        selected_room: Option<u32>,
    ) -> Self {
        let mut view = Self {
            time_slots: Vec::new(),
            room_order: Vec::new(),
            grid_events: Vec::new(),
            collapsed_times: Vec::new(),
            schedule,
            selected_day,
            selected_room,
        };

        view.generate_time_slots();
        view.generate_collapsed_times();
        view.generate_room_order();
        view.generate_grid_events();
        view
    }

    fn generate_time_slots(&mut self) {
        let events = self.get_filtered_events();
        
        if events.is_empty() {
            return;
        }

        if let (Some(first_event), Some(last_event)) = (events.first(), events.last()) {
            // Start at the beginning of the hour of the first event
            let mut current_time = first_event
                .start_time
                .with_minute(0)
                .unwrap_or(first_event.start_time);
            let end_time = last_event
                .end_time
                .with_minute(0)
                .unwrap_or(last_event.end_time)
                + chrono::Duration::hours(1);

            while current_time < end_time {
                let end_time_slot = current_time + chrono::Duration::minutes(30);
                self.time_slots.push(TimeSlot {
                    key: current_time.format("%Y%m%d_%H%M").to_string(),
                    start_time: current_time,
                    end_time: end_time_slot,
                });
                current_time = end_time_slot;
            }
        }
    }

    fn generate_collapsed_times(&mut self) {
        let events = self.get_filtered_events();
        
        if events.is_empty() {
            return;
        }

        // Group events by day
        let mut events_by_day: std::collections::HashMap<NaiveDate, Vec<&Event>> = std::collections::HashMap::new();
        for event in &events {
            let date = event.date();
            events_by_day.entry(date).or_default().push(event);
        }
        
        // Sort days
        let mut sorted_days: Vec<_> = events_by_day.keys().cloned().collect();
        sorted_days.sort();
        
        let mut collapsed_times = Vec::new();
        
        for (day_idx, &day) in sorted_days.iter().enumerate() {
            // Skip if this is the last day (no overnight time after)
            if day_idx == sorted_days.len() - 1 {
                continue;
            }
            
            let day_events = &events_by_day[&day];
            let next_day = sorted_days[day_idx + 1];
            let next_day_events = &events_by_day[&next_day];
            
            if day_events.is_empty() || next_day_events.is_empty() {
                continue;
            }
            
            // Find last event on current day (excluding SPLIT events)
            let last_event = day_events
                .iter()
                .filter(|e| !self.is_split_event(e))
                .max_by_key(|e| e.end_time)
                .unwrap_or_else(|| day_events.iter().max_by_key(|e| e.end_time).unwrap());
            let collapse_start = last_event.end_time + chrono::Duration::minutes(30); // 30 minutes after last event
            
            // Find first event on next day (excluding SPLIT events)
            let first_event_next = next_day_events
                .iter()
                .filter(|e| !self.is_split_event(e))
                .min_by_key(|e| e.start_time)
                .unwrap_or_else(|| next_day_events.iter().min_by_key(|e| e.start_time).unwrap());
            let collapse_end = first_event_next.start_time; // End exactly at first event

            // Only create collapsible time if there's a significant gap
            if collapse_end > collapse_start {
                let key = format!(
                    "collapse_{}_{}",
                    collapse_start.format("%Y%m%d_%H%M"),
                    collapse_end.format("%Y%m%d_%H%M")
                );
                collapsed_times.push(CollapsedTime {
                    key,
                    start_time: collapse_start,
                    end_time: collapse_end,
                    is_collapsed: true, // Default to collapsed
                    label: format!("{} – {}", collapse_start.format("%-I:%M %p"), collapse_end.format("%-I:%M %p")),
                });
            }
        }
        
        self.collapsed_times = collapsed_times;
    }

    fn generate_room_order(&mut self) {
        let mut rooms: BTreeSet<u32> = BTreeSet::new();
        
        for event in self.get_filtered_events() {
            if let Some(room_id) = event.room_id {
                rooms.insert(room_id);
            }
        }
        
        self.room_order = rooms.into_iter().collect();
    }

    fn generate_grid_events(&mut self) {
        self.grid_events.clear();
        
        let events: Vec<_> = self.get_filtered_events().into_iter().cloned().collect();
        
        for event in events {
            if let Some(room_id) = event.room_id {
                if let (Some(start_slot), Some(end_slot)) = (
                    self.find_slot_index(event.start_time),
                    self.find_slot_index(event.end_time),
                ) {
                    let panel_type = event
                        .panel_type
                        .as_ref()
                        .and_then(|pt_uid| self.schedule.panel_type_by_prefix(pt_uid));
                    
                    let color = panel_type
                        .and_then(|pt| pt.color.clone())
                        .and_then(|color_str| {
                            // Parse hex color string to u32
                            if let Some(hex) = color_str.strip_prefix('#') {
                                u32::from_str_radix(hex, 16).ok()
                            } else {
                                None
                            }
                        })
                        .unwrap_or(0x3B82F6); // Default blue color

                    self.grid_events.push(GridEvent {
                        event: event.clone(),
                        room_id,
                        start_slot,
                        end_slot,
                        panel_type: panel_type.cloned(),
                        color,
                    });
                }
            }
        }
    }

    fn find_slot_index(&self, time: NaiveDateTime) -> Option<usize> {
        self.time_slots
            .iter()
            .position(|slot| slot.start_time <= time && time < slot.end_time)
    }

    fn get_filtered_events(&self) -> Vec<&Event> {
        let events: Vec<&Event> = self.schedule.events.iter().collect();
        
        let events = if let Some(selected_day) = self.selected_day {
            events
                .into_iter()
                .filter(|event| event.date() == selected_day)
                .collect()
        } else {
            events
        };

        if let Some(selected_room) = self.selected_room {
            events
                .into_iter()
                .filter(|event| event.room_id == Some(selected_room))
                .collect()
        } else {
            events
        }
    }

    fn is_break_event(&self, event: &Event) -> bool {
        if let Some(pt_uid) = &event.panel_type {
            if let Some(panel_type) = self
                .schedule
                .panel_types
                .iter()
                .find(|pt| pt.effective_uid() == *pt_uid)
            {
                return panel_type.is_break;
            }
        }
        false
    }

    fn is_split_event(&self, event: &Event) -> bool {
        if let Some(pt_uid) = &event.panel_type {
            if let Some(panel_type) = self
                .schedule
                .panel_types
                .iter()
                .find(|pt| pt.effective_uid() == *pt_uid)
            {
                return panel_type.prefix.to_uppercase() == "SPLIT";
            }
        }
        false
    }
}

#[allow(refining_impl_trait)]
impl Render for GridView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> Stateful<Div> {
        let bg = rgb(0xFFFFFF);
        let border_color = rgb(0xE5E7EB);
        let header_bg = rgb(0xF9FAFB);
        let time_color = rgb(0x6B7280);
        let empty_color = rgb(0x9CA3AF);

        let events = self.get_filtered_events();

        if events.is_empty() {
            return div()
                .flex()
                .flex_col()
                .flex_grow()
                .bg(bg)
                .id("empty-state")
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .justify_center()
                        .items_center()
                        .py(px(48.0))
                        .text_color(empty_color)
                        .child("No events for this selection")
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(0x6B7280))
                                .child("Try selecting a different day or removing room filters"),
                        ),
                );
        }

        // Generate time slots with collapsible overnight periods
        let mut all_time_slots = Vec::new();
        
        // Group events by day for overnight collapse logic
        let mut events_by_day: std::collections::HashMap<NaiveDate, Vec<&Event>> = std::collections::HashMap::new();
        for event in &events {
            let date = event.date();
            events_by_day.entry(date).or_default().push(event);
        }
        
        // Sort days
        let mut sorted_days: Vec<_> = events_by_day.keys().cloned().collect();
        sorted_days.sort();
        
        for (day_idx, &day) in sorted_days.iter().enumerate() {
            let day_events = &events_by_day[&day];
            
            if day_events.is_empty() {
                continue;
            }
            
            // Find first and last event times for this day
            let first_event = day_events.iter().min_by_key(|e| e.start_time).unwrap();
            let last_event = day_events.iter().max_by_key(|e| e.end_time).unwrap();
            
            // Start at the beginning of the hour of the first event
            let mut current_time = first_event.start_time.with_minute(0).unwrap_or(first_event.start_time);
            let day_end = last_event.end_time.with_minute(0).unwrap_or(last_event.end_time) + chrono::Duration::hours(1);
            
            // Add time slots for this day
            while current_time < day_end {
                all_time_slots.push(current_time);
                current_time = current_time + chrono::Duration::minutes(30);
            }
            
            // Check if we need to add collapsible overnight time (except after last day)
            if day_idx < sorted_days.len() - 1 {
                let next_day = sorted_days[day_idx + 1];
                let next_day_events = &events_by_day[&next_day];
                
                if !next_day_events.is_empty() {
                    // Find first event on next day (excluding SPLIT events)
                    let first_event_next = next_day_events
                        .iter()
                        .filter(|e| !self.is_split_event(e))
                        .min_by_key(|e| e.start_time)
                        .unwrap_or_else(|| next_day_events.iter().min_by_key(|e| e.start_time).unwrap());
                    let collapse_start = last_event.end_time + chrono::Duration::minutes(30);
                    let collapse_end = first_event_next.start_time; // End exactly at first event
                    
                    if collapse_end > collapse_start {
                        // Find the corresponding collapsed time entry
                        if let Some(collapsed_time) = self
                            .collapsed_times
                            .iter()
                            .find(|ct| ct.start_time == collapse_start)
                        {
                            if collapsed_time.is_collapsed {
                                // Add just the collapse indicator
                                all_time_slots.push(collapse_start);
                            } else {
                                // Add all the time slots when expanded
                                let mut expanded_time = collapse_start;
                                while expanded_time < collapse_end {
                                    all_time_slots.push(expanded_time);
                                    expanded_time = expanded_time + chrono::Duration::minutes(30);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Create scrollable grid
        let total_rooms = self.room_order.len();
        let total_slots = all_time_slots.len();
        
        // Build all grid content first
        let mut grid_content = Vec::new();
        
        // Add time header (top-left corner)
        grid_content.push(
            div()
                .bg(header_bg)
                .border_b_2()
                .border_r_1()
                .border_color(border_color)
                .p(px(8.0))
                .text_sm()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(0x374151))
                .flex()
                .items_center()
                .justify_center()
                .child("Time"),
        );

        // Add room headers
        for &room_id in &self.room_order {
            if let Some(room) = self.schedule.room_by_id(room_id) {
                grid_content.push(
                    div()
                        .bg(header_bg)
                        .border_b_2()
                        .border_l_1()
                        .border_color(border_color)
                        .p(px(4.0))
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(0x374151))
                        .flex()
                        .items_center()
                        .justify_center()
                        .overflow_hidden()
                        .child(room.long_name.clone()),
                );
            }
        }

        // Add grid cells for each time slot
        let mut rendered_row_to_slot_index = Vec::new();
        let mut last_displayed_day: Option<NaiveDate> = None;
        
        for (row_idx, slot_time) in all_time_slots.iter().enumerate() {
            // Map rendered row to actual slot index in self.time_slots
            if let Some(actual_slot_idx) = self.find_slot_index(*slot_time) {
                rendered_row_to_slot_index.push(actual_slot_idx);
            } else {
                // For collapsed indicators, use the next available slot
                rendered_row_to_slot_index.push(self.time_slots.len());
            }
            
            // Check if this is a collapsible time indicator
            let is_collapsible_indicator = self.collapsed_times.iter().any(|ct| {
                ct.start_time == *slot_time && ct.is_collapsed
            });

            // Determine if we should show day indicator
            let current_day = slot_time.date();
            let show_day_indicator = last_displayed_day.is_none() || 
                last_displayed_day != Some(current_day) ||
                (self.selected_day.is_none() && row_idx == 0); // First entry on "All Days"

            if is_collapsible_indicator {
                // Collapsible indicator row - time cell with day indicator if needed
                let collapsed_time = self
                    .collapsed_times
                    .iter()
                    .find(|ct| ct.start_time == *slot_time)
                    .unwrap();
                
                let time_cell_content = if show_day_indicator {
                    let day_name = current_day.format("%A, %B %d").to_string();
                    format!("{} ⏸ {}", day_name, collapsed_time.label)
                } else {
                    format!("⏸ {}", collapsed_time.label)
                };
                
                grid_content.push(
                    div()
                        .bg(rgb(0xF0FDF4))
                        .border_b_1()
                        .border_r_1()
                        .border_color(rgb(0xD1D5DB))
                        .p(px(8.0))
                        .text_xs()
                        .text_color(rgb(0x059669))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .flex()
                        .items_center()
                        .child(time_cell_content),
                );

                // Update last displayed day
                if show_day_indicator {
                    last_displayed_day = Some(current_day);
                }

                // Single clickable cell spanning all rooms
                let total_rooms = self.room_order.len();
                grid_content.push(
                    div()
                        // .id(format!("collapse_{}", row_idx).as_str())
                        .bg(rgb(0xF9FAFB))
                        .border_b_1()
                        .border_l_1()
                        .border_color(rgb(0xD1D5DB))
                        .col_span(total_rooms as u16)
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .hover(|style| style.bg(rgb(0xF3F4F6)))
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x6B7280))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .child("Click to expand overnight hours"),
                        ),
                );
            } else {
                // Regular time slot row - time cell with day indicator if needed
                let time_str = slot_time.format("%-I:%M %p").to_string();
                let is_hour = slot_time.minute() == 0;

                let time_cell_content = if show_day_indicator {
                    let day_name = current_day.format("%A, %B %d").to_string();
                    format!("{} • {}", day_name, time_str)
                } else {
                    time_str
                };

                grid_content.push(
                    div()
                        .bg(bg)
                        .border_b_1()
                        .border_r_1()
                        .border_color(border_color)
                        .p(px(8.0))
                        .text_xs()
                        .text_color(time_color)
                        .font_weight(if is_hour || show_day_indicator {
                            gpui::FontWeight::SEMIBOLD
                        } else {
                            gpui::FontWeight::NORMAL
                        })
                        .flex()
                        .items_center()
                        .child(time_cell_content),
                );

                // Update last displayed day
                if show_day_indicator {
                    last_displayed_day = Some(current_day);
                }

                // Room cells for this time slot
                for &room_id in &self.room_order {
                    let mut cell = div()
                        .bg(bg)
                        .border_b_1()
                        .border_l_1()
                        .border_color(border_color)
                        .relative();

                    // Check if there's an event in this room at this time
                    // Use the mapped slot index for proper event positioning
                    let actual_slot_idx = rendered_row_to_slot_index[row_idx];
                    if let Some(grid_event) = self.grid_events.iter().find(|ge| {
                        ge.room_id == room_id && ge.start_slot <= actual_slot_idx && actual_slot_idx < ge.end_slot
                    }) {
                        let event = &grid_event.event;
                        let color = grid_event.color;

                        let event_card = div()
                            .absolute()
                            .top(px(-1.0)) // Cover top border
                            .left(px(-1.0)) // Cover left border
                            .right(px(-1.0)) // Cover right border
                            .bottom(px(-1.0)) // Cover bottom border
                            .bg(rgb(color))
                            .border_1()
                            .border_color(rgb(color))
                            .rounded(px(4.0))
                            .p(px(4.0))
                            .cursor_pointer()
                            .hover(|style| style.opacity(0.8));

                        let event_content = div()
                            .text_xs()
                            .text_color(rgb(0xFFFFFF))
                            .child(event.name.clone());

                        cell = cell.child(event_card.child(event_content));
                    }

                    grid_content.push(cell);
                }
            }
        }

        // Create the scrollable grid with all content
        div()
            .w_full()
            .h_full()
            .id("grid-container")
            .overflow_scroll()
            .child(
                div()
                    .grid()
                    .bg(bg)
                    .w_full()
                    .h(px((total_slots * 60) as f32)) // Fixed height based on content
                    .grid_cols((total_rooms + 1) as u16) // Time column + room columns
                    .grid_rows((total_slots + 1) as u16) // Header row + time slot rows
                    .children(grid_content)
            )
    }
}

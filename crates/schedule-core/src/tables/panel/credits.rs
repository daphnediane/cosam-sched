/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Credit computation for panel presenters.
//!
//! This module handles the complex logic of computing formatted presenter
//! credit strings, accounting for group resolution, subsumption, and various
//! override flags.

use crate::entity::EntityUuid;
use crate::schedule::Schedule;
use crate::tables::panel::{
    PanelEntityType, PanelId, EDGE_CREDITED_PRESENTERS, EDGE_UNCREDITED_PRESENTERS,
};
use crate::tables::presenter::{self, PresenterCommonData, PresenterEntityType, PresenterId};
use std::collections::{HashMap, HashSet};

/// State of a presenter in the credit computation process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreditState {
    /// Presenter is credited directly.
    Credited,
    /// Presenter is shown individually (due to `show_individually` flag).
    ShownIndividually,
    /// Presenter is subsumed by a group.
    Subsumed,
}

/// Holds the state and context for credit computation.
pub struct CreditCalculator<'a> {
    schedule: &'a Schedule,
    panel_id: PanelId,
    presenter_lookup: HashMap<PresenterId, &'a PresenterCommonData>,
    all_presenters: HashSet<PresenterId>,
    credited_ids: Vec<PresenterId>,
    listing_state: HashMap<PresenterId, CreditState>,
    credit_strings: HashMap<PresenterId, Option<String>>,
    groups_with_all_members: HashSet<PresenterId>,
}

impl<'a> CreditCalculator<'a> {
    /// Create a new credit calculator for the given panel.
    pub fn new(schedule: &'a Schedule, panel_id: PanelId) -> Option<Self> {
        let panel_internal = schedule.get_internal::<PanelEntityType>(panel_id)?;

        // Check for hide_panelist override
        if panel_internal.data.hide_panelist {
            return None;
        }

        // Get credited presenters directly from the credited edge list
        let credited_ids: Vec<PresenterId> = schedule
            .connected_field_nodes(panel_id, EDGE_CREDITED_PRESENTERS)
            .into_iter()
            .map(|e| unsafe { PresenterId::new_unchecked(e.entity_uuid()) })
            .collect();

        if credited_ids.is_empty() {
            return None;
        }

        // Get all presenters using inclusive logic
        let all_presenters = get_inclusive_presenters(schedule, panel_id);

        // Build a schedule-wide lookup so group entities not directly on this
        // panel (e.g. referenced only via always_grouped membership) can be found.
        let presenter_lookup: HashMap<PresenterId, &PresenterCommonData> = schedule
            .iter_entities::<PresenterEntityType>()
            .map(|(id, internal)| (id, &internal.data))
            .collect();

        Some(Self {
            schedule,
            panel_id,
            presenter_lookup,
            all_presenters,
            credited_ids,
            listing_state: HashMap::new(),
            credit_strings: HashMap::new(),
            groups_with_all_members: HashSet::new(),
        })
    }

    /// Check for alt_panelist override and return it if present.
    pub fn check_alt_panelist_override(&self) -> Option<String> {
        let panel_internal = self
            .schedule
            .get_internal::<PanelEntityType>(self.panel_id)?;
        panel_internal.data.alt_panelist.clone()
    }

    /// Determine the initial credit state for all presenters.
    ///
    /// First pass: walks through credited presenters and determines whether each
    /// should be credited directly, always shown, or subsumed by a group.
    pub fn determine_credit_states(&mut self) {
        let mut to_check: Vec<PresenterId> = self.credited_ids.clone();

        while let Some(presenter_id) = to_check.pop() {
            if self.listing_state.contains_key(&presenter_id) {
                continue;
            }

            let Some(presenter_data) = self.presenter_lookup.get(&presenter_id) else {
                continue;
            };

            if presenter_data.show_individually {
                // Member with show_individually appears individually, not subsumed by group
                self.listing_state
                    .insert(presenter_id, CreditState::ShownIndividually);
            } else {
                self.check_for_group_subsumption(presenter_id, presenter_data, &mut to_check);
            }
        }
    }

    /// Check if a presenter should be subsumed by any of its groups.
    fn check_for_group_subsumption(
        &mut self,
        presenter_id: PresenterId,
        _presenter_data: &PresenterCommonData,
        to_check: &mut Vec<PresenterId>,
    ) {
        let group_ids: Vec<PresenterId> = self
            .schedule
            .connected_field_nodes(presenter_id, presenter::EDGE_GROUPS)
            .into_iter()
            .map(|e| unsafe { PresenterId::new_unchecked(e.entity_uuid()) })
            .collect();

        let mut should_subsume = false;
        for &group_id in &group_ids {
            if let Some(group_data) = self.presenter_lookup.get(&group_id) {
                let group_should_show = self.should_show_group(group_id, group_data);

                if group_should_show {
                    should_subsume = true;
                    to_check.push(group_id);
                }
            }
        }

        let state = if should_subsume {
            CreditState::Subsumed
        } else {
            CreditState::Credited
        };
        self.listing_state.insert(presenter_id, state);
    }

    /// Determine if a group should be shown (either subsumes_members or all members present).
    fn should_show_group(&self, group_id: PresenterId, group_data: &PresenterCommonData) -> bool {
        if group_data.subsumes_members {
            return true;
        }

        let group_member_ids: Vec<PresenterId> = self
            .schedule
            .connected_field_nodes(group_id, presenter::EDGE_MEMBERS)
            .into_iter()
            .map(|e| unsafe { PresenterId::new_unchecked(e.entity_uuid()) })
            .collect();

        group_member_ids
            .iter()
            .all(|m| self.all_presenters.contains(m))
    }

    /// Handle group member subsumption for all presenters.
    ///
    /// Second pass: processes ALL presenters (including subsumed) to check if any
    /// are groups with all members present, and marks those members as subsumed.
    pub fn handle_group_member_subsumption(&mut self) {
        let presenter_ids: Vec<PresenterId> = self.listing_state.keys().copied().collect();

        for presenter_id in presenter_ids {
            let member_ids: Vec<PresenterId> = self
                .schedule
                .connected_field_nodes(presenter_id, presenter::EDGE_MEMBERS)
                .into_iter()
                .map(|e| unsafe { PresenterId::new_unchecked(e.entity_uuid()) })
                .collect();

            if member_ids.is_empty() {
                continue;
            }

            let all_members_present = member_ids.iter().all(|m| self.all_presenters.contains(m));

            if all_members_present {
                self.groups_with_all_members.insert(presenter_id);
                for &member_id in &member_ids {
                    // Only subsume members that are already in listing_state
                    // AND don't have show_individually set
                    if let Some(&state) = self.listing_state.get(&member_id) {
                        if state != CreditState::ShownIndividually {
                            self.listing_state.insert(member_id, CreditState::Subsumed);
                        }
                    }
                }
            }
        }
    }

    /// Compute the final credit strings for all non-subsumed presenters.
    ///
    /// Uses a work queue algorithm with dependency resolution to handle
    /// groups whose credit strings depend on their members' credits.
    pub fn compute_credit_strings(&mut self) {
        let mut work_queue: Vec<PresenterId> = self
            .listing_state
            .iter()
            .filter_map(|(&id, &state)| {
                if state != CreditState::Subsumed {
                    Some(id)
                } else {
                    None
                }
            })
            .collect();

        let mut deferred_queue: Vec<PresenterId> = Vec::new();
        let mut forward_progress_made = false;

        loop {
            if work_queue.is_empty() {
                if deferred_queue.is_empty() || !forward_progress_made {
                    break;
                }
                work_queue.append(&mut deferred_queue);
                forward_progress_made = false;
                continue;
            }

            let presenter_id = work_queue.remove(0);

            if self.credit_strings.contains_key(&presenter_id) {
                continue;
            }

            let Some(presenter_data) = self.presenter_lookup.get(&presenter_id).copied() else {
                continue;
            };

            let member_ids: Vec<PresenterId> = self
                .schedule
                .connected_field_nodes(presenter_id, presenter::EDGE_MEMBERS)
                .into_iter()
                .map(|e| unsafe { PresenterId::new_unchecked(e.entity_uuid()) })
                .collect();

            if member_ids.is_empty() || self.groups_with_all_members.contains(&presenter_id) {
                // No members or all members present - can compute immediately
                self.credit_strings
                    .insert(presenter_id, Some(presenter_data.name.clone()));
                work_queue.append(&mut deferred_queue);
                forward_progress_made = true;
                continue;
            }

            self.try_compute_group_credit(
                presenter_id,
                presenter_data,
                &member_ids,
                &mut work_queue,
                &mut deferred_queue,
                &mut forward_progress_made,
            );
        }

        self.handle_remaining_deferred(deferred_queue);
    }

    /// Try to compute credit for a group, or defer if members aren't ready.
    fn try_compute_group_credit(
        &mut self,
        presenter_id: PresenterId,
        presenter_data: &PresenterCommonData,
        member_ids: &[PresenterId],
        work_queue: &mut Vec<PresenterId>,
        deferred_queue: &mut Vec<PresenterId>,
        forward_progress_made: &mut bool,
    ) {
        let mut members_need_credits = false;

        for &member_id in member_ids {
            if !self.credit_strings.contains_key(&member_id)
                && self
                    .listing_state
                    .get(&member_id)
                    .is_some_and(|&state| state != CreditState::Subsumed)
            {
                members_need_credits = true;

                if !work_queue.contains(&member_id) && !deferred_queue.contains(&member_id) {
                    work_queue.push(member_id);
                }
            }
        }

        if members_need_credits {
            if !deferred_queue.contains(&presenter_id) {
                deferred_queue.push(presenter_id);
            }
        } else {
            let credit_string = self.build_group_credit_string(presenter_data, member_ids);
            self.credit_strings.insert(presenter_id, credit_string);
            work_queue.append(deferred_queue);
            *forward_progress_made = true;
        }
    }

    /// Build the credit string for a group based on its credited members.
    fn build_group_credit_string(
        &self,
        presenter_data: &PresenterCommonData,
        member_ids: &[PresenterId],
    ) -> Option<String> {
        // Include members who are either credited directly or subsumed (always_grouped)
        let relevant_members: Vec<PresenterId> = member_ids
            .iter()
            .filter(|m| {
                self.credited_ids.contains(m)
                    || self
                        .listing_state
                        .get(m)
                        .is_some_and(|&s| s == CreditState::Subsumed)
            })
            .cloned()
            .collect();

        if relevant_members.len() == 1 {
            // Single member - show "Member of Group"
            // Use member's credit string if available, fallback to member name
            let member_name = relevant_members
                .first()
                .and_then(|m| self.credit_strings.get(m))
                .and_then(|opt| opt.clone())
                .or_else(|| {
                    relevant_members
                        .first()
                        .and_then(|m| self.presenter_lookup.get(m))
                        .map(|d| d.name.clone())
                })?;
            Some(format!("{} of {}", member_name, presenter_data.name))
        } else {
            // Multiple members - show "Group (Member1, Member2)"
            // Use each member's credit string if available, fallback to member name
            let names: Vec<String> = relevant_members
                .iter()
                .filter_map(|m| {
                    self.credit_strings
                        .get(m)
                        .and_then(|opt| opt.clone())
                        .or_else(|| self.presenter_lookup.get(m).map(|d| d.name.clone()))
                })
                .collect();

            if names.is_empty() {
                Some(presenter_data.name.clone())
            } else {
                Some(format!("{} ({})", presenter_data.name, names.join(", ")))
            }
        }
    }

    /// Handle any remaining deferred items by falling back to simple names.
    fn handle_remaining_deferred(&mut self, deferred_queue: Vec<PresenterId>) {
        for presenter_id in deferred_queue {
            if !self.credit_strings.contains_key(&presenter_id) {
                if let Some(presenter_data) = self.presenter_lookup.get(&presenter_id) {
                    self.credit_strings
                        .insert(presenter_id, Some(presenter_data.name.clone()));
                }
            }
        }
    }

    /// Generate the final sorted list of credit strings.
    pub fn into_credits(self) -> Vec<String> {
        let mut credit_presenters: Vec<(PresenterId, &PresenterCommonData)> = self
            .listing_state
            .iter()
            .filter_map(|(&id, &state)| {
                if state != CreditState::Subsumed {
                    self.presenter_lookup.get(&id).map(|&data| (id, data))
                } else {
                    None
                }
            })
            .collect();

        // Sort by presenter rank and name
        credit_presenters.sort_by(|a, b| {
            a.1.rank
                .priority()
                .cmp(&b.1.rank.priority())
                .then_with(|| a.1.name.cmp(&b.1.name))
        });

        // Generate credits in sorted order
        credit_presenters
            .into_iter()
            .filter_map(|(presenter_id, presenter_data)| {
                self.credit_strings
                    .get(&presenter_id)
                    .and_then(|opt| opt.clone())
                    .or_else(|| Some(presenter_data.name.clone()))
            })
            .collect()
    }
}

/// Get inclusive presenters for a panel (direct + transitive groups + transitive members)
pub fn get_inclusive_presenters(schedule: &Schedule, panel_id: PanelId) -> HashSet<PresenterId> {
    let credited_ids: Vec<PresenterId> = schedule
        .connected_field_nodes(panel_id, EDGE_CREDITED_PRESENTERS)
        .into_iter()
        .map(|e| unsafe { PresenterId::new_unchecked(e.entity_uuid()) })
        .collect();

    let uncredited_ids: Vec<PresenterId> = schedule
        .connected_field_nodes(panel_id, EDGE_UNCREDITED_PRESENTERS)
        .into_iter()
        .map(|e| unsafe { PresenterId::new_unchecked(e.entity_uuid()) })
        .collect();

    let direct: Vec<PresenterId> = credited_ids.into_iter().chain(uncredited_ids).collect();
    let mut result: HashSet<PresenterId> = HashSet::new();

    for p in direct {
        result.insert(p);

        // Inclusive members of p: all members of p (following EDGE_MEMBERS from p)
        for m in schedule
            .inclusive_edges::<PresenterEntityType, PresenterEntityType>(p, presenter::EDGE_MEMBERS)
        {
            result.insert(m);
        }

        // Inclusive groups of p: all groups p belongs to (following EDGE_GROUPS from p)
        for g in schedule
            .inclusive_edges::<PresenterEntityType, PresenterEntityType>(p, presenter::EDGE_GROUPS)
        {
            result.insert(g);
        }
    }

    result
}

/// Compute the formatted presenter credit strings for `panel_id`.
///
/// Applies `hide_panelist` / `alt_panelist` overrides, then filters to
/// credited presenters (per the per-edge `credited` bool), then formats each
/// credit entry accounting for groups, `always_shown_in_group`, and
/// `always_grouped` members.
///
/// The presenter lookup is built from **all** presenters in the schedule so
/// that group entities that are not themselves panel edges can still be
/// resolved for name formatting.
pub fn compute_credits(schedule: &Schedule, panel_id: PanelId) -> Vec<String> {
    // Check for alt_panelist override first
    if let Some(alt) = CreditCalculator::new(schedule, panel_id)
        .as_ref()
        .and_then(|calc| calc.check_alt_panelist_override())
    {
        return vec![alt];
    }

    let Some(mut calculator) = CreditCalculator::new(schedule, panel_id) else {
        return Vec::new();
    };

    calculator.determine_credit_states();
    calculator.handle_group_member_subsumption();
    calculator.compute_credit_strings();
    calculator.into_credits()
}

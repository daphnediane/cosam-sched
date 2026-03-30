/*
 * Copyright (c) 2026 Daphne Pfister
 * SPDX-License-Identifier: BSD-2-Clause
 * See LICENSE file for full license text
 */

//! Presenter entity implementation

use crate::EntityFields;
use std::fmt;

/// Presenter ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PresenterId(u64);

impl fmt::Display for PresenterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "presenter-{}", self.0)
    }
}

/// Presenter entity with EntityFields derive macro
#[derive(EntityFields, Debug, Clone)]
pub struct Presenter {
    #[field(display = "Name", description = "Presenter's full name")]
    #[alias("name", "full_name", "display_name")]
    #[indexable(priority = 200)]
    #[required]
    pub name: String,

    #[field(display = "Rank", description = "Presenter's rank or title")]
    #[alias("rank", "title", "position")]
    pub rank: Option<String>,

    #[field(
        display = "Sort Rank Column",
        description = "Sorting rank for column layout"
    )]
    #[alias("sort_rank_column", "column_rank", "col_sort")]
    pub sort_rank_column: Option<i64>,

    #[field(display = "Sort Rank Row", description = "Sorting rank for row layout")]
    #[alias("sort_rank_row", "row_rank", "row_sort")]
    pub sort_rank_row: Option<i64>,

    #[field(
        display = "Sort Rank Member",
        description = "Sorting rank for member layout"
    )]
    #[alias("sort_rank_member", "member_rank", "member_sort")]
    pub sort_rank_member: Option<i64>,

    #[field(
        display = "Always Shown",
        description = "Whether presenter is always visible"
    )]
    #[alias("always_shown", "always_visible", "permanent")]
    pub always_shown: bool,

    #[field(display = "Bio", description = "Presenter's biography")]
    #[alias("bio", "biography", "description")]
    pub bio: Option<String>,

    #[field(display = "Email", description = "Presenter's email address")]
    #[alias("email", "email_address", "contact_email")]
    pub email: Option<String>,

    #[field(display = "Phone", description = "Presenter's phone number")]
    #[alias("phone", "phone_number", "contact_phone")]
    pub phone: Option<String>,

    // @TODO: Implement edge support for is_group, members, groups, always_grouped, always_shown_in_group

    // @TODO: Not currently in the spreadsheets, Windsurf thought this was a good idea
    // I agree but we currently don't have the data
    #[field(display = "Pronouns", description = "Presenter's preferred pronouns")]
    #[alias("pronouns", "preferred_pronouns")]
    pub pronouns: Option<String>,

    // @TODO: Not currently in the spreadsheets, Windsurf thought this was a good idea
    // I agree but we currently don't have the data
    #[field(display = "Website", description = "Presenter's website")]
    #[alias("website", "url", "web", "site")]
    pub website: Option<String>,
}

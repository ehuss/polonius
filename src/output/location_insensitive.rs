// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::collections::BTreeSet;
use std::time::Instant;

use crate::facts::{AllFacts, Loan, Point, Region};
use crate::output::Output;

use datafrog::{Iteration, Relation};

pub(super) fn compute(dump_enabled: bool, mut all_facts: AllFacts) -> Output {
    let all_points: BTreeSet<Point> = all_facts
        .cfg_edge
        .iter()
        .map(|&(p, _)| p)
        .chain(all_facts.cfg_edge.iter().map(|&(_, q)| q))
        .collect();

    for &r in &all_facts.universal_region {
        for &p in &all_points {
            all_facts.region_live_at.push((r, p));
        }
    }

    let mut result = Output::new(dump_enabled);

    let borrow_live_at_start = Instant::now();

    let borrow_live_at = {
        // Create a new iteration context, ...
        let mut iteration = Iteration::new();

        // .. some variables, ..
        let subset = iteration.variable::<(Region, Region)>("subset");
        let requires = iteration.variable::<(Region, Loan)>("requires");
        let borrow_live_at = iteration.variable::<(Loan, Point)>("borrow_live_at");
        let region_live_at = iteration.variable::<(Region, Point)>("region_live_at");

        // load initial facts.

        // subset(R1, R2) :- outlives(R1, R2, _P)
        subset.insert(Relation::from(
            all_facts.outlives.iter().map(|&(r1, r2, _p)| (r1, r2)),
        ));

        // requires(R, B) :- borrow_region(R, B, _P).
        requires.insert(Relation::from(
            all_facts.borrow_region.iter().map(|&(r, b, _p)| (r, b)),
        ));

        region_live_at.insert(all_facts.region_live_at.into());

        // .. and then start iterating rules!
        while iteration.changed() {
            // requires(R2, B) :- requires(R1, B), subset(R1, R2).
            requires.from_join(&requires, &subset, |&_r1, &b, &r2| (r2, b));

            // borrow_live_at(B, P) :- requires(R, B), region_live_at(R, P)
            borrow_live_at.from_join(&requires, &region_live_at, |&_r, &b, &p| (b, p));
        }

        borrow_live_at.complete()
    };

    if dump_enabled {
        println!(
            "borrow_live_at is complete: {} tuples, {:?}",
            borrow_live_at.len(),
            borrow_live_at_start.elapsed()
        );
    }

    for (borrow, location) in &borrow_live_at.elements {
        result
            .borrow_live_at
            .entry(*location)
            .or_insert(Vec::new())
            .push(*borrow);
    }

    result
}

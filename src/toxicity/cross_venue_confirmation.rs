use crate::types::{flow::VenueFlowBreakdown, market::Venue, toxic::ToxicDirection};

pub fn cross_venue_confirmed(
    direction: ToxicDirection,
    venue_breakdown: &std::collections::BTreeMap<String, VenueFlowBreakdown>,
    min_venues: usize,
) -> bool {
    same_direction_venue_count(direction, venue_breakdown) >= min_venues
}

pub fn same_direction_venue_count(
    direction: ToxicDirection,
    venue_breakdown: &std::collections::BTreeMap<String, VenueFlowBreakdown>,
) -> usize {
    Venue::ALL
        .into_iter()
        .filter(|venue| {
            venue_breakdown
                .get(venue.as_key())
                .is_some_and(|breakdown| venue_confirms_direction(direction, breakdown))
        })
        .count()
}

pub fn leader_venue(
    direction: ToxicDirection,
    venue_breakdown: &std::collections::BTreeMap<String, VenueFlowBreakdown>,
) -> Option<Venue> {
    if direction == ToxicDirection::Neutral {
        return None;
    }

    Venue::ALL.into_iter().max_by(|left, right| {
        direction_volume(direction, venue_breakdown, *left).total_cmp(&direction_volume(
            direction,
            venue_breakdown,
            *right,
        ))
    })
}

fn venue_confirms_direction(direction: ToxicDirection, breakdown: &VenueFlowBreakdown) -> bool {
    match direction {
        ToxicDirection::Buy => {
            breakdown.net_aggressive_btc > 0.0 && breakdown.aggressive_buy_btc > 0.0
        }
        ToxicDirection::Sell => {
            breakdown.net_aggressive_btc < 0.0 && breakdown.aggressive_sell_btc > 0.0
        }
        ToxicDirection::Neutral => false,
    }
}

fn direction_volume(
    direction: ToxicDirection,
    venue_breakdown: &std::collections::BTreeMap<String, VenueFlowBreakdown>,
    venue: Venue,
) -> f64 {
    let Some(breakdown) = venue_breakdown.get(venue.as_key()) else {
        return 0.0;
    };
    match direction {
        ToxicDirection::Buy => breakdown.aggressive_buy_btc,
        ToxicDirection::Sell => breakdown.aggressive_sell_btc,
        ToxicDirection::Neutral => 0.0,
    }
}

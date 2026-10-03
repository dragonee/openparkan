//! The order a distance-sorted layer of the render queue draws its items in.
//!
//! A layer of sort type 1 or 3 keeps its items farthest first (docs/11, "Effect sprites are
//! drawn far to near"): the add slot both types share (`Terrain.dll:0x1003e090`) asks each
//! item how far it stands from the camera and puts it into the list by a search between the
//! list's two ends, and the render walks the list from its top. Group 1's layer 5, the
//! see-through surfaces, is type 1; layers 6 and 7, the effects, are type 3, which draws the
//! same order through shared vertex buffers.

/// The items of one sorted layer in the order they are drawn, given how far each stands
/// from the camera, in the order they were filed.
///
/// The search is the layer's own, because it decides what a tie does. It probes the middle of
/// what is left of the list: an item nearer than the probe goes on below it, a farther one
/// above it, and one **exactly as far takes the probe's place**, ahead of the item it tied
/// with (`0x1003e0fa`–`0x1003e161`). Where the search closes on one item, the new one goes
/// below it only if it is nearer (`0x1003e169`–`0x1003e193`). So of two items at one distance
/// the one filed later is drawn first, and a longer run of equals is not left in any one
/// order: two sprites on one matrix draw the second under the first.
///
/// A palm is some two hundred see-through batches and a map holds palms by the hundred, so
/// the list is not walked to place an item: what the search meets at a probe follows from
/// how many items are farther than the new one and how many are as far, which are counted.
pub fn far_to_near(distances: &[f32]) -> Vec<u32> {
    // Every distance there is, farthest first, and each item's place among them.
    let mut steps: Vec<f32> = distances.to_vec();
    steps.sort_unstable_by(|a, b| b.total_cmp(a));
    steps.dedup_by(|a, b| a.total_cmp(b).is_eq());
    let step_of: Vec<usize> =
        distances.iter().map(|d| steps.partition_point(|s| s.total_cmp(d).is_gt())).collect();
    // The drawn list is its steps' runs end to end, so each run's place in it is known before
    // anything is filed: where it starts, and how much of it is filled.
    let mut starts = vec![0usize; steps.len() + 1];
    for &step in &step_of {
        starts[step + 1] += 1;
    }
    for step in 0..steps.len() {
        starts[step + 1] += starts[step];
    }
    let mut held = vec![0usize; steps.len()];
    let mut filed = Counts::new(steps.len());
    let mut list = vec![0u32; distances.len()];
    for (item, &step) in step_of.iter().enumerate() {
        let farther = filed.before(step);
        let at = starts[step] + place(item, farther, farther + held[step]) - farther;
        let end = starts[step] + held[step];
        list.copy_within(at..end, at + 1);
        list[at] = item as u32;
        held[step] += 1;
        filed.add(step);
    }
    list
}

/// Where the layer's search puts a new item into a list of `len`, farthest first, whose
/// items `equal_from..equal_to` are exactly as far as it is: everything before them is
/// farther and everything after nearer.
fn place(len: usize, equal_from: usize, equal_to: usize) -> usize {
    let Some(last) = len.checked_sub(1) else { return 0 };
    // The ends of what is left; `high` has crossed `low` once it is below it.
    let (mut low, mut high) = (0usize, last as isize);
    while high > low as isize {
        let probe = (low + high as usize) / 2;
        if probe < equal_from {
            low = probe + 1;
        } else if probe >= equal_to {
            high = probe as isize - 1;
        } else {
            return probe;
        }
    }
    if high == low as isize && low < equal_from { low + 1 } else { low }
}

/// How many items stand at each of a row of steps, with the sum before any step: a binary
/// indexed tree.
struct Counts(Vec<u32>);

impl Counts {
    fn new(steps: usize) -> Self {
        Self(vec![0; steps + 1])
    }

    fn add(&mut self, step: usize) {
        let mut at = step + 1;
        while at < self.0.len() {
            self.0[at] += 1;
            at += at & at.wrapping_neg();
        }
    }

    /// How many items stand at the steps before `step`.
    fn before(&self, step: usize) -> usize {
        let (mut at, mut sum) = (step, 0usize);
        while at > 0 {
            sum += self.0[at] as usize;
            at &= at - 1;
        }
        sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The search as the layer runs it, on the list itself: an item at a time, each compared
    /// with the item at the probe.
    fn searched(distances: &[f32]) -> Vec<u32> {
        let mut list: Vec<u32> = Vec::new();
        for (item, &distance) in distances.iter().enumerate() {
            let far = |at: usize| distances[list[at] as usize];
            let mut at = 0;
            if let Some(last) = list.len().checked_sub(1) {
                let (mut low, mut high) = (0usize, last as isize);
                let mut tied = None;
                while high > low as isize {
                    let probe = (low + high as usize) / 2;
                    if distance == far(probe) {
                        tied = Some(probe);
                        break;
                    }
                    if distance < far(probe) {
                        low = probe + 1;
                    } else {
                        high = probe as isize - 1;
                    }
                }
                at = match tied {
                    Some(probe) => probe,
                    None if high == low as isize && distance < far(low) => low + 1,
                    None => low,
                };
            }
            list.insert(at, item as u32);
        }
        list
    }

    #[test]
    fn a_sorted_layer_draws_far_to_near_whatever_order_its_items_were_filed_in() {
        assert_eq!(far_to_near(&[]), Vec::<u32>::new());
        assert_eq!(far_to_near(&[7.0]), [0]);
        // Filed near, far, middle: drawn far, middle, near.
        assert_eq!(far_to_near(&[50.0, 300.0, 120.0]), [1, 2, 0]);
        // Every insertion lands in order, at either end and in the middle.
        let filed = [10.0, 8.0, 6.0, 4.0, 2.0, 5.0, 7.0, 9.0, 12.0, 1.0, 3.0, 11.0];
        let drawn: Vec<f32> = far_to_near(&filed).iter().map(|&i| filed[i as usize]).collect();
        let mut sorted = filed.to_vec();
        sorted.sort_by(|a, b| b.total_cmp(a));
        assert_eq!(drawn, sorted);
    }

    /// The search puts an item that ties with its probe in the probe's place, so the one filed
    /// later is drawn first.
    #[test]
    fn of_two_items_at_one_distance_the_one_filed_later_is_drawn_first() {
        assert_eq!(far_to_near(&[5.0, 5.0]), [1, 0]);
        // Whether the tie is met by a probe or where the search closes: beside a farther item,
        // beside a nearer one, and between the two.
        assert_eq!(far_to_near(&[9.0, 5.0, 5.0]), [0, 2, 1]);
        assert_eq!(far_to_near(&[5.0, 1.0, 5.0]), [2, 0, 1]);
        assert_eq!(far_to_near(&[9.0, 5.0, 1.0, 5.0]), [0, 3, 1, 2]);
        // A run of equals is not left in one order: the fourth goes in at the probe, under
        // the third, and the fifth under it again.
        assert_eq!(far_to_near(&[5.0; 3]), [2, 1, 0]);
        assert_eq!(far_to_near(&[5.0; 4]), [2, 3, 1, 0]);
        assert_eq!(far_to_near(&[5.0; 5]), [2, 4, 3, 1, 0]);
    }

    /// Counting what a probe would meet gives the list the search itself leaves, ties and
    /// all: over lists of every length to 40 with few distances, and long ones with many.
    #[test]
    fn the_counted_order_is_the_one_the_search_itself_leaves() {
        let mut state = 0x2545_f491_u32;
        let mut draw = |below: u32| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state % below
        };
        for len in 0..40 {
            for spread in [1, 2, 3, 5, 9] {
                let filed: Vec<f32> = (0..len).map(|_| draw(spread) as f32).collect();
                assert_eq!(far_to_near(&filed), searched(&filed), "{filed:?}");
            }
        }
        for spread in [7, 100, 100_000] {
            let filed: Vec<f32> = (0..700).map(|_| draw(spread) as f32 * 0.25).collect();
            assert_eq!(far_to_near(&filed), searched(&filed));
        }
    }
}

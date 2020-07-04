use std::vec::Vec;
use std::collections::HashSet;

/// Weak k-compositions that sum up to <n
pub fn compositions(n: u8, k: usize) -> Vec<Vec<u8>>  {
    if k == 0 {
        return vec![vec![]];
    }
    let tails = compositions(n, k - 1);

    let mut output = Vec::new();

    for tail in tails {
        let cap = n - tail.iter().sum::<u8>();
        for x in 0..=cap {
            let mut new_tail = tail.clone();
            new_tail.insert(0, x);
            if new_tail.iter().sum::<u8>() < n {
                assert!(new_tail.len() == k, "new_tail is not a k-composition");
                output.push(new_tail);
            }
        }
    }
    return output;
}

/// Actions up to some cap
pub fn ranked_actions(cap: &Vec<u8>) -> Vec<HashSet<Vec<u8>>> {

    let upper = cap.iter().sum();
    // ordered [1..upper]
    let mut result = Vec::with_capacity(cap.iter().sum::<u8>() as usize);

    for i in 1..=upper {
        result.push(go(cap, i));
    }

    
    fn go(cap: &Vec<u8>, n: u8) -> HashSet<Vec<u8>> {
        let sum: u8 = cap.iter().sum();

        let mut output: HashSet<Vec<u8>> = HashSet::new();

        // only one 0-sum
        if n == 0 {
            output.insert(vec![0; cap.len()]);
            return output;
        }

        // only one (sum cap)-sum
        if n == sum {
            output.insert(cap.to_vec());
            return output;
        }

        let previous = go(cap, n-1);

        for elem in previous {
            for i in 0..cap.len() {
                let mut new_elem = elem.clone();
                if new_elem[i] < cap[i] {
                    new_elem[i] += 1;
                    output.insert(new_elem);
                }
            }
        }
        output
    };

    result
}
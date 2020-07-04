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

/// Actions up to some cap, verified by a valid function
pub fn ranked_actions<F>(len: usize, upper: u8, valid: F) -> Vec<HashSet<Vec<u8>>> 
    where F: Fn(usize, &Vec<u8>) -> bool {

    // ordered [1..upper]
    let mut result = Vec::with_capacity(upper as usize);

    for i in 1..=upper {
        result.push(go(len, upper, &valid, i));
    }

    
    fn go<F>(len: usize, upper: u8, valid: &F, n: u8) -> HashSet<Vec<u8>>
        where F: Fn(usize, &Vec<u8>) -> bool {

        let mut output: HashSet<Vec<u8>> = HashSet::new();

        // only one 0-sum
        if n == 0 {
            output.insert(vec![0; len]);
            return output;
        }

        let previous = go(len, upper, valid, n-1);

        for elem in previous {
            for i in 0..len {
                let mut new_elem = elem.clone();
                if valid(i, &new_elem) {
                    new_elem[i] += 1;
                    output.insert(new_elem);
                }
            }
        }
        output
    };

    result
}
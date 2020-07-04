use std::vec::Vec;

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
pub fn ranked_actions(cap: &Vec<u8>) -> Vec<Vec<Vec<u8>>> {

    // sorry i have the terminal haskell disease :(
    fn go(cap: &Vec<u8>, n: u8) -> Vec<Vec<Vec<u8>>> {
        let sum: u8 = cap.iter().sum();
        if sum == n { 
            return vec![ vec![ cap.clone() ] ];
        }

        let mut output: Vec<Vec<Vec<u8>>> = Vec::new();

        let cur_rank: Vec<Vec<u8>> = compositions(n+1, cap.len()).into_iter()
                .filter(|xs| xs.iter().sum::<u8>() == n)
                .filter(|xs| xs.iter().zip(cap).all(|(a,b)| *a <= *b))
                .collect();
        
        output.push(cur_rank);

        output.append(&mut go(cap, n+1));
        output
    };

    go(cap, 1)
}
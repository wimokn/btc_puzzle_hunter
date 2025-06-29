use btc_puzzle_hunter::calculate_walk_parameters;

fn main() {
    println!("🧪 Random Walk Parameter Calculation Test\n");
    
    let test_cases = vec![
        ("Raspberry Pi", 1_000),
        ("Old Laptop", 5_000),
        ("Basic Desktop", 25_000),
        ("Gaming PC", 100_000),
        ("High-End Workstation", 500_000),
        ("GPU Mining Rig", 2_000_000),
    ];
    
    println!("{:<25} {:<12} {:<10} {:<8} {:<15} {:<15}", 
             "Machine Type", "Keys/Sec", "Iterations", "Walks", "Adapt Int.", "Est. Runtime");
    println!("{}", "-".repeat(85));
    
    for (name, performance) in test_cases {
        let (iterations, walks, adapt_interval) = calculate_walk_parameters(performance);
        let total_keys = iterations * walks;
        let est_runtime = total_keys as f64 / performance as f64;
        
        println!("{:<25} {:<12} {:<10} {:<8} {:<15} {:.1}s", 
                 name, 
                 format!("{}", performance),
                 format!("{}", iterations),
                 walks,
                 format!("{}", adapt_interval),
                 est_runtime);
    }
    
    println!("\n📊 Analysis:");
    println!("• Slower machines get longer walks with fewer parallel threads");
    println!("• Faster machines get shorter walks with more parallel threads");
    println!("• Adaptation intervals scale to maintain good exploration balance");
    println!("• All configurations target reasonable runtime (5-60 seconds)");
}
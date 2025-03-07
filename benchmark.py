import subprocess
import re
import numpy as np
import matplotlib.pyplot as plt
from tqdm import tqdm
import os
from datetime import datetime
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle

def run_benchmark(min_res, max_res, step, d, nb_call_per_res):
    """
    Run benchmarks for different resolutions and plot the results.
    
    Args:
        min_res: Starting resolution
        max_res: Maximum resolution
        step: Step size for resolution increment
        d: Value for the -d parameter
        nb_call_per_res: Number of times to run each resolution for averaging
    """
    resolutions = list(range(min_res, max_res + 1, step))
    avg_times = []
    all_times = []  # Store all measured times for candlestick plotting
    
    # Create timestamp for unique filenames
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    
    print(f"Running benchmarks from resolution {min_res} to {max_res} with step {step}")
    print(f"Each resolution will be tested {nb_call_per_res} times")
    
    # Ensure the output directory exists
    os.makedirs("benched_graphs", exist_ok=True)
    
    for res in tqdm(resolutions):
        times = []
        
        for _ in range(nb_call_per_res):
            cmd = f"cargo run --release -- --reso {res} --jfa-mode gpu --no-mesh-visualization --plot none -d {d}"
            result = subprocess.run(cmd, shell=True, text=True, capture_output=True)
            
            # Extract the elapsed time from the output
            ms_match = re.search(r"Time elapsed: ([\d.]+)ms", result.stdout)
            s_match = re.search(r"Time elapsed: ([\d.]+)s", result.stdout)
            
            if ms_match:
                time_ms = float(ms_match.group(1))
                times.append(time_ms)
            elif s_match:
                time_ms = float(s_match.group(1)) * 1000  # Convert seconds to milliseconds
                times.append(time_ms)
            else:
                time_ms = None
        
        if times:
            all_times.append(times)  # Store all times for this resolution
            avg_time = sum(times) / len(times)
            avg_times.append(avg_time)
            print(f"Resolution {res}: Average time = {avg_time:.2f}ms")
        else:
            print(f"Resolution {res}: Failed to extract timing information")
            avg_times.append(0)
            all_times.append([])
    
    # Generate output filenames with timestamp
    plot_filename = f"benched_graphs/benchmark_d{d}_{timestamp}.png"
    csv_filename = f"benched_graphs/benchmark_d{d}_{timestamp}.csv"
    
    # Plot the results with candlestick chart
    fig, ax = plt.figure(figsize=(12, 8)), plt.gca()
    
    # Plot average line
    ax.plot(resolutions, avg_times, marker='o', color='blue', label='Average')
    
    # Add candlestick bars for distribution
    for i, (res, times) in enumerate(zip(resolutions, all_times)):
        if not times:
            continue
            
        # Calculate statistics
        min_time = min(times)
        max_time = max(times)
        q1 = np.percentile(times, 25)
        q3 = np.percentile(times, 75)
        median = np.median(times)
        
        # Draw candlestick
        # Box
        rect = Rectangle((res - step/4, q1), step/2, q3-q1, 
                         fill=True, color='skyblue', alpha=0.5)
        ax.add_patch(rect)
        
        # Median line
        ax.hlines(median, res-step/4, res+step/4, colors='blue', linewidth=2)
        
        # Min/Max whiskers
        ax.vlines(res, min_time, max_time, colors='black', linestyle='-')
        
        # Horizontal whisker caps
        whisker_width = step/8
        ax.hlines(min_time, res-whisker_width, res+whisker_width, colors='black')
        ax.hlines(max_time, res-whisker_width, res+whisker_width, colors='black')
    
    plt.title(f"Performance Benchmark JFA_wgpu (d={d})")
    plt.xlabel("Resolution")
    plt.ylabel("Execution Time (ms)")
    plt.grid(True)
    plt.legend()
    
    # Save the plot and display it
    plt.savefig(plot_filename)
    plt.show()
    
    # Save the data to a CSV file with parameters at the beginning
    with open(csv_filename, "w") as f:
        # Write benchmark parameters at the top
        f.write("# Benchmark Parameters\n")
        f.write(f"# Minimum Resolution: {min_res}\n")
        f.write(f"# Maximum Resolution: {max_res}\n")
        f.write(f"# Step Size: {step}\n")
        f.write(f"# D Parameter: {d}\n")
        f.write(f"# Runs Per Resolution: {nb_call_per_res}\n")
        f.write(f"# Timestamp: {timestamp}\n")
        f.write("#\n")
        
        # Write header
        f.write("Resolution,Average Time (ms),Min Time (ms),Max Time (ms),Median Time (ms),Q1 Time (ms),Q3 Time (ms)")
        
        # Write individual run times as additional columns
        for i in range(nb_call_per_res):
            f.write(f",Run {i+1} (ms)")
        f.write("\n")
        
        # Write data rows
        for res, avg_time, times in zip(resolutions, avg_times, all_times):
            if times:
                min_time = min(times)
                max_time = max(times)
                median = np.median(times)
                q1 = np.percentile(times, 25)
                q3 = np.percentile(times, 75)
                
                f.write(f"{res},{avg_time},{min_time},{max_time},{median},{q1},{q3}")
                
                # Add all individual run times
                for t in times:
                    f.write(f",{t}")
                
                # If we have fewer actual times than nb_call_per_res, add empty cells
                for _ in range(nb_call_per_res - len(times)):
                    f.write(",")
            else:
                # Write row with empty/zero values
                f.write(f"{res},{avg_time},0,0,0,0,0")
                
                # Add empty cells for individual runs
                for _ in range(nb_call_per_res):
                    f.write(",")
            
            f.write("\n")
    
    print(f"Results saved to {plot_filename} and {csv_filename}")

if __name__ == "__main__":
    import argparse
    
    parser = argparse.ArgumentParser(description='Benchmark blue noise generator at different resolutions')
    parser.add_argument('--min_res', type=int, required=True, help='Minimum resolution')
    parser.add_argument('--max_res', type=int, required=True, help='Maximum resolution')
    parser.add_argument('--step', type=int, required=True, help='Resolution step size')
    parser.add_argument('--d', type=float, required=True, help='d parameter value')
    parser.add_argument('--nb_call_per_res', type=int, required=True, help='Number of runs per resolution')
    
    args = parser.parse_args()
    
    run_benchmark(
        args.min_res,
        args.max_res,
        args.step,
        args.d,
        args.nb_call_per_res
    )
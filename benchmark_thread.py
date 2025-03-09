import subprocess
import re
import numpy as np
import matplotlib.pyplot as plt
from tqdm import tqdm
import os
from datetime import datetime
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle

def run_benchmark(resolution, min_threads, max_threads, step_threads, d, nb_call_per_thread):
    """
    Run benchmarks for different thread counts and plot the results.
    
    Args:
        resolution: Fixed resolution to use for all benchmarks
        min_threads: Minimum number of threads
        max_threads: Maximum number of threads
        step_threads: Step size for thread count increment
        d: Value for the -d parameter
        nb_call_per_thread: Number of times to run each thread count for averaging
    """
    thread_counts = list(range(min_threads, max_threads + 1, step_threads))
    avg_times = []
    all_times = []  # Store all measured times for candlestick plotting
    
    # Create timestamp for unique filenames
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    
    print(f"Running benchmarks with resolution {resolution} for thread counts from {min_threads} to {max_threads} with step {step_threads}")
    print(f"Each thread count will be tested {nb_call_per_thread} times")
    
    # Ensure the output directory exists
    os.makedirs("benched_graphs", exist_ok=True)
    
    for thread_count in tqdm(thread_counts):
        times = []
        
        for _ in range(nb_call_per_thread):
            cmd = f"cargo run --release -- --reso {resolution} --threads {thread_count} --jfa-mode gpu --no-mesh-visualization --plot none -d {d}"
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
            all_times.append(times)  # Store all times for this thread count
            avg_time = sum(times) / len(times)
            avg_times.append(avg_time)
            print(f"Thread count {thread_count}: Average time = {avg_time:.2f}ms")
        else:
            print(f"Thread count {thread_count}: Failed to extract timing information")
            avg_times.append(0)
            all_times.append([])
    
    # Generate output filenames with timestamp
    plot_filename = f"benched_graphs/thread_benchmark_res{resolution}_d{d}_{timestamp}.png"
    csv_filename = f"benched_graphs/thread_benchmark_res{resolution}_d{d}_{timestamp}.csv"
    
    # Convert milliseconds to seconds for plotting
    avg_times_seconds = [t/1000 for t in avg_times]
    all_times_seconds = [[t/1000 for t in times] for times in all_times]
    
    # Plot the results with candlestick chart
    fig, ax = plt.figure(figsize=(12, 8)), plt.gca()
    
    # Plot average line
    ax.plot(thread_counts, avg_times_seconds, marker='o', color='blue', label='Average')
    
    # Add candlestick bars for distribution
    for i, (thread_count, times) in enumerate(zip(thread_counts, all_times_seconds)):
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
        rect = Rectangle((thread_count - step_threads/4, q1), step_threads/2, q3-q1, 
                         fill=True, color='skyblue', alpha=0.5)
        ax.add_patch(rect)
        
        # Median line
        ax.hlines(median, thread_count-step_threads/4, thread_count+step_threads/4, colors='blue', linewidth=2)
        
        # Min/Max whiskers
        ax.vlines(thread_count, min_time, max_time, colors='black', linestyle='-')
        
        # Horizontal whisker caps
        whisker_width = step_threads/8
        ax.hlines(min_time, thread_count-whisker_width, thread_count+whisker_width, colors='black')
        ax.hlines(max_time, thread_count-whisker_width, thread_count+whisker_width, colors='black')
    
    plt.title(f"Thread Performance Benchmark JFA_wgpu (Resolution={resolution}, d={d})")
    plt.xlabel("Number of Threads")
    plt.ylabel("Execution Time (seconds)")
    plt.grid(True)
    plt.legend()
    
    # Save the plot and display it
    plt.savefig(plot_filename)
    plt.show()
    
    # Save the data to a CSV file with parameters at the beginning
    with open(csv_filename, "w") as f:
        # Write benchmark parameters at the top
        f.write("# Benchmark Parameters\n")
        f.write(f"# Resolution: {resolution}\n")
        f.write(f"# Minimum Threads: {min_threads}\n")
        f.write(f"# Maximum Threads: {max_threads}\n")
        f.write(f"# Step Size: {step_threads}\n")
        f.write(f"# D Parameter: {d}\n")
        f.write(f"# Runs Per Thread Count: {nb_call_per_thread}\n")
        f.write(f"# Timestamp: {timestamp}\n")
        f.write("#\n")
        
        # Write header
        f.write("Thread Count,Average Time (s),Min Time (s),Max Time (s),Median Time (s),Q1 Time (s),Q3 Time (s)")
        
        # Write individual run times as additional columns
        for i in range(nb_call_per_thread):
            f.write(f",Run {i+1} (s)")
        f.write("\n")
        
        # Write data rows
        for thread_count, avg_time, times in zip(thread_counts, avg_times_seconds, all_times_seconds):
            if times:
                min_time = min(times)
                max_time = max(times)
                median = np.median(times)
                q1 = np.percentile(times, 25)
                q3 = np.percentile(times, 75)
                
                f.write(f"{thread_count},{avg_time},{min_time},{max_time},{median},{q1},{q3}")
                
                # Add all individual run times
                for t in times:
                    f.write(f",{t}")
                
                # If we have fewer actual times than nb_call_per_thread, add empty cells
                for _ in range(nb_call_per_thread - len(times)):
                    f.write(",")
            else:
                # Write row with empty/zero values
                f.write(f"{thread_count},{avg_time},0,0,0,0,0")
                
                # Add empty cells for individual runs
                for _ in range(nb_call_per_thread):
                    f.write(",")
            
            f.write("\n")
    
    print(f"Results saved to {plot_filename} and {csv_filename}")

if __name__ == "__main__":
    import argparse
    
    parser = argparse.ArgumentParser(description='Benchmark blue noise generator with different thread counts')
    parser.add_argument('--resolution', type=int, required=True, help='Fixed resolution to use')
    parser.add_argument('--min_threads', type=int, required=True, help='Minimum thread count')
    parser.add_argument('--max_threads', type=int, required=True, help='Maximum thread count')
    parser.add_argument('--step_threads', type=int, required=True, help='Thread count step size')
    parser.add_argument('--d', type=float, required=True, help='d parameter value')
    parser.add_argument('--nb_call_per_thread', type=int, required=True, help='Number of runs per thread count')
    
    args = parser.parse_args()
    
    run_benchmark(
        args.resolution,
        args.min_threads,
        args.max_threads,
        args.step_threads,
        args.d,
        args.nb_call_per_thread
    )
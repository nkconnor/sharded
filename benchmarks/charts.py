

import json
import pandas as pd


results = "latest.txt"

rows = []

with open(results) as fh:
    for line in fh:
        """{
            "level": "INFO",
            "message": "generating operation mix",
            "span": {
                "mix": "Mix { read: 94, insert: 2, remove: 1, update: 3, upsert: 0 }",
                "name": "benchmark",
                "threads": 1
            },
            "spans": [
                {
                    "name": "task",
                    "task": "read_heavy"
                },
                {
                    "name": "trial_num",
                    "trial_num": 0
                },
                {
                    "kind": "main::adapters::ContrieTable<u64>",
                    "name": "kind"
                },
                {
                    "mix": "Mix { read: 94, insert: 2, remove: 1, update: 3, upsert: 0 }",
                    "name": "benchmark",
                    "threads": 1
                }
            ],
            "target": "bustle",
            "timestamp": "Aug 19 13:25:47.159"
        }"""
        line = json.loads(line)
        if line.get("avg") == None: continue
        
        rows.append({
            "timestamp": line["timestamp"],
            "impl": line["spans"][2]["kind"],
            "task": line["spans"][0]["task"],
            "trial": line["spans"][1]["trial_num"],
            "threads": line["span"]["threads"],
            "mix": line["span"]["mix"],
            "avg": int(line["avg"].strip('ns')),
            "ops": line["ops"],
            "took": line["took"],
            "_debug": line["message"]  
        })
        
df = pd.DataFrame.from_records(rows)

import matplotlib.pyplot as plt
import matplotlib


font = {'family' : 'normal',
        'weight' : 'normal',
        'size'   : 22}

matplotlib.rc('font', **font)


df = df[df['impl'] != 'main::adapters::CHashMapTable<u64>']
df = df[df['impl'] != 'main::adapters::MutexStdTable<u64>']
#df = df[df['task'] == "update"]
title_set = True
i = 0

for task, task_df in df.groupby('task'):
    print(task)
    
    fig, ax = plt.subplots(figsize=(16,12))
    
    title_string = task.replace("_", " ")
    title_string = title_string.title()
    
    
    for label, group in task_df.groupby('impl'):
        
        see = group[['threads', 'avg']]
        see = see.groupby('threads').mean().reset_index()
        see.plot(
            x="avg", 
            y="threads", 
            ax=ax, 
            label=label.strip("main::adapters::").strip("<u64>"), 
            #title=f"Average Performance ({task})", 
            style='x', # '.--'
            ms=10
        )
    
    yticks = list(filter(lambda x: x % 2 == 0, list(df["threads"].unique())))
    
  
    subtitle_string = f"random seeds={5}; params=default"
    
    fig.suptitle(title_string, y=0.97, fontsize=26)
    title_set = True
        
    plt.title(subtitle_string, y=1.01, fontsize=22) #, fontsize=10)
    
    ax.set_yticks(yticks)
    ax.set_ylabel("threads", fontsize=24, labelpad=20)
    ax.set_xlabel("avg ns / op", fontsize=24, labelpad=20)
    plt.legend()
    fig.savefig(f"avg_performance_{task}.png")

    i += 1

# Timetrack Jr.

Small CLI utility to facilitate tracking the time it takes to do different activities (at work, for consultants, or whatever!).

Timetrack Jr. logs the start and end times of different activities to a local sqlite database and can export summaries of those time logs as csv, json, a text summary, or an .ical file (that you could pull into your favorite calendar tool)

## Quick Start
First, set up your ttjr database with some categories to record times to.  Optionally, you can set an end-of-day time to automaticall end times (I forget to press "stop" at the end of the work day)
```sh
#########
# DB Setup
#########
#time is tracked under different categories
$ ttjr add-category project-for-client-a
$ ttjr add-category project-for-client-b
#if you want, you can set an end-of-day time which will automatically end any started time categories at 17:00
$ ttjr set-option end-of-day 17:00
$ ttjr show-config
{
  "options": {
    "dbversion": "0.1.0",
    "end-of-day": "17:00"
  },
  "categories": [
    "project-for-client-a",
    "project-for-client-b"
  ]
}
#by default, times will be saved to an sqlite db in the current directory
$ ls
ttjr.sqlite3
#If you want it somewhere else, use --db-path like
$ ttjr --db-path ~/.ttjr.sqlite3 <COMMAND>

######
# Start timing stuff!
######
#start working on something, add --notify to fire a desktop notification, useful if you bind `start-timing` commands to global keyboard shortcuts
$ ttjr start-timing project-for-client-a --notify
#hopefully do some work for a while...
#start working on something else (no need to explicitly stop timing)
$ ttjr start-timing project-for-client-b
#go get a sandwich
$ ttjr stop-timing 
#back to work little capitalist
$ ttjr start-timing project-for-client-a



######
# Export timing data to do something interesting with it!
######
#quick summary - use exact dates or human-readable date-like strings to constrain exports
$ ttjr export --format summary --start-time "14 days ago"
Tabulating results starting on/after Fri, 28 Oct 2022 19:18:02 -0400
Logged 3 activites for a total of 03:00
project-for-client-a:
  2 logs, 01:00 cumulative, 33.33% of total
project-for-client-b:
  1 logs, 02:00 cumulative, 66.67% of total

#Export time data as json, csv, or ical
$ ttjr export --format json
[
  {
    "id": 1,
    "category": "project-for-client-a",
    "start_time": 1667307600,
    "end_time": 1667311200
  },
  {
    "id": 2,
    "category": "project-for-client-b",
    "start_time": 1667311200,
    "end_time": 1667318400
  },
  {
    "id": 3,
    "category": "project-for-client-a",
    "start_time": 1667322000,
    "end_time": null
  }
]

#Have ttjr generate and keep up-to-date an ical file that you can pull into gcal/outlook/etc
$ ttjr export --format ical --outfile ~/my_times.ical --listen


######
# Editing/amending times
######
#amend an entry in case you started or stopped it at the wrong time (or made it the wrong category)
$ ttjr amend-time 2 -s "2022-11-01 10:00" -e "2022-11-01 12:00"
#delete an entry
$ ttjr delete-time 3

```
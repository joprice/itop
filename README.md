# itop

## Notes

- In the github action, restoring the cache shows errors like "actions cache github Cannot utime: Operation not permitted is:issue". This is a known issue: https://github.com/actions/cache/issues/133

## TODO

- only recalculate highlighted item on process list change
  - this avoids a selected reference hanging around
  - move selection to nearest neighbor when item dies?
  - set selected on process meta item

# My system
## Data lake Architecture
### RAW
#### What to keep:
- file data
- unstable API data
- API with rate limit
#### Directory structure
Stooq:
!!! Use hive style partitioning....
`raw/stooq/symbol=<symbol>/year=<y>/month=<m>/day=<d>/<timestamp>.csv`

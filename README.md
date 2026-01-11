# My system
## Data lake Architecture
### RAW
#### What to keep:
- file data
- unstable API data
- API with rate limit
#### Directory structure
Stooq:
`raw/stooq/<symbol>/<interval>/<date>/<timestamp>.csv`

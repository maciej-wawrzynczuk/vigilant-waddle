import polars as pl


def main():
    ads = SilverStookOhlcv("ads.de")
    print(ads.data.schema)

class SilverStookOhlcv:
    def __init__(self, symbol: str):
        self.symbol = symbol 
        self.data = (
            pl.read_csv(self.__stooq_url())
            .with_columns([
                pl.col("Date").cast(pl.Utf8).str.to_date(format="%Y-%m-%d"),
                pl.col(["Open", "High", "Low", "Close"]).cast(pl.Float64),
                pl.col("Volume").cast(pl.Float64)
            ]).sort("Date")
        )

    def __stooq_url(self) -> str:
        return f"https://stooq.com/q/d/l/?s={self.symbol}&i=d"

if __name__ == "__main__":
    main()

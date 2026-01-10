import polars as pl


def main():
    ads = SilverStookOhlcv("ads.de")
    print(ads.data.describe())

class SilverStookOhlcv:
    def __init__(self, symbol: str):
        self.symbol = symbol 
        self.data = pl.read_csv(self.__stooq_url())

    def __stooq_url(self) -> str:
        return f"https://stooq.com/q/d/l/?s={self.symbol}&i=d"

if __name__ == "__main__":
    main()

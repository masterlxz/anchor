class Quote {
  final String ticker;
  final double price;
  final String? name;
  final String? exchange;
  final String? currency;

  /// `coin_id` do CoinGecko por trás deste `ticker` — só preenchido por
  /// `CoinGeckoQuoteService`, pra `Asset.externalId` guardar no cadastro e
  /// pular a busca por texto nos próximos refreshes.
  final String? externalId;

  const Quote({
    required this.ticker,
    required this.price,
    this.name,
    this.exchange,
    this.currency,
    this.externalId,
  });

  /// Espelha o parsing de `chart.result[0].meta` em
  /// `data-collector/sources/acoes_yahoo.py::fetch_quotes` — mesmos campos,
  /// mesma tolerância a campos ausentes (só `regularMarketPrice` é
  /// obrigatório).
  factory Quote.fromYahooMeta(String ticker, Map<String, dynamic> meta) {
    return Quote(
      ticker: ticker,
      price: (meta['regularMarketPrice'] as num).toDouble(),
      name: (meta['longName'] ?? meta['shortName']) as String?,
      exchange: (meta['fullExchangeName'] ?? meta['exchangeName']) as String?,
      currency: meta['currency'] as String?,
    );
  }
}

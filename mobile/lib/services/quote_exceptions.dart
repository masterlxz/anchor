/// Lançada por qualquer serviço de cotação (`YahooQuoteService`,
/// `CoinGeckoQuoteService`) quando o ticker/símbolo não resolve.
class QuoteNotFoundException implements Exception {
  final String ticker;
  QuoteNotFoundException(this.ticker);
}

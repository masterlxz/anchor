import 'dart:convert';

import 'package:http/http.dart' as http;

import '../models/quote.dart';

class QuoteNotFoundException implements Exception {
  final String ticker;
  QuoteNotFoundException(this.ticker);
}

/// Cliente Dart do mesmo endpoint não-oficial do Yahoo Finance já usado (e
/// confirmado contra a API real) pelo `data-collector` em
/// `sources/acoes_yahoo.py::fetch_quotes` — público, sem chave. Ver a nota
/// nesse arquivo Python sobre a rota não ser documentada oficialmente pelo
/// Yahoo (mesma usada por baixo dos panos pela lib `yfinance`).
class YahooQuoteService {
  static const _chartUrl = 'https://query1.finance.yahoo.com/v8/finance/chart';

  /// Sufixo `.SA` fixo (B3) — mesmo default do `data-collector`.
  Future<Quote> fetchQuote(String ticker) async {
    final uri = Uri.parse('$_chartUrl/$ticker.SA').replace(queryParameters: {
      'range': '5d',
      'interval': '1d',
    });

    final response = await http.get(
      uri,
      headers: {'User-Agent': 'Mozilla/5.0'},
    ).timeout(const Duration(seconds: 15));

    if (response.statusCode != 200) {
      throw QuoteNotFoundException(ticker);
    }

    try {
      final body = jsonDecode(response.body) as Map<String, dynamic>;
      final result = (body['chart']['result'] as List).first as Map<String, dynamic>;
      final meta = result['meta'] as Map<String, dynamic>;
      return Quote.fromYahooMeta(ticker, meta);
    } catch (_) {
      throw QuoteNotFoundException(ticker);
    }
  }
}

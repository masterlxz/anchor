import 'package:flutter/material.dart';

import '../models/asset.dart';
import '../services/portfolio_repository.dart';
import '../services/yahoo_quote_service.dart';

class AddAssetScreen extends StatefulWidget {
  const AddAssetScreen({super.key});

  @override
  State<AddAssetScreen> createState() => _AddAssetScreenState();
}

class _AddAssetScreenState extends State<AddAssetScreen> {
  final _tickerController = TextEditingController();
  final _nameController = TextEditingController();
  final _quoteService = YahooQuoteService();
  final _repository = PortfolioRepository();

  AssetClass _assetClass = AssetClass.acaoBr;
  String _currency = 'BRL';
  bool _searching = false;
  bool _saving = false;
  String? _error;

  Future<void> _search() async {
    final ticker = _tickerController.text.trim().toUpperCase();
    if (ticker.isEmpty) return;

    setState(() {
      _searching = true;
      _error = null;
    });

    try {
      final quote = await _quoteService.fetchQuote(ticker);
      setState(() {
        _nameController.text = quote.name ?? ticker;
        _currency = quote.currency ?? 'BRL';
      });
    } catch (_) {
      setState(() => _error = 'Não foi possível buscar $ticker.SA — confira o ticker.');
    } finally {
      setState(() => _searching = false);
    }
  }

  Future<void> _save() async {
    final ticker = _tickerController.text.trim().toUpperCase();
    final name = _nameController.text.trim();
    if (ticker.isEmpty || name.isEmpty) return;

    setState(() => _saving = true);

    await _repository.insertAsset(Asset(
      ticker: ticker,
      name: name,
      assetClass: _assetClass,
      currency: _currency,
      createdAt: DateTime.now(),
    ));

    if (mounted) Navigator.of(context).pop(true);
  }

  @override
  void dispose() {
    _tickerController.dispose();
    _nameController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Adicionar ativo')),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _tickerController,
                    textCapitalization: TextCapitalization.characters,
                    decoration: const InputDecoration(
                      labelText: 'Ticker (B3)',
                      hintText: 'PETR4',
                    ),
                  ),
                ),
                const SizedBox(width: 12),
                OutlinedButton(
                  onPressed: _searching ? null : _search,
                  child: const Text('Buscar'),
                ),
              ],
            ),
            if (_error != null) ...[
              const SizedBox(height: 8),
              Text(_error!, style: TextStyle(color: Theme.of(context).colorScheme.error)),
            ],
            const SizedBox(height: 16),
            TextField(
              controller: _nameController,
              decoration: const InputDecoration(labelText: 'Nome'),
            ),
            const SizedBox(height: 16),
            DropdownButtonFormField<AssetClass>(
              initialValue: _assetClass,
              decoration: const InputDecoration(labelText: 'Classe'),
              items: AssetClass.values
                  .map((c) => DropdownMenuItem(value: c, child: Text(c.label)))
                  .toList(),
              onChanged: (value) {
                if (value != null) setState(() => _assetClass = value);
              },
            ),
            const SizedBox(height: 24),
            FilledButton(
              onPressed: _saving ? null : _save,
              child: const Text('Salvar'),
            ),
          ],
        ),
      ),
    );
  }
}

import 'package:flutter/material.dart';

import '../models/asset.dart';
import '../services/portfolio_repository.dart';
import '../services/quote_dispatcher.dart';

class AddAssetScreen extends StatefulWidget {
  const AddAssetScreen({super.key});

  @override
  State<AddAssetScreen> createState() => _AddAssetScreenState();
}

class _AddAssetScreenState extends State<AddAssetScreen> {
  final _tickerController = TextEditingController();
  final _nameController = TextEditingController();
  final _dispatcher = QuoteDispatcher();
  final _repository = PortfolioRepository();

  AssetClass _assetClass = AssetClass.acaoBr;
  String _currency = AssetClass.acaoBr.defaultCurrency;
  String? _externalId;
  bool _searching = false;
  bool _saving = false;
  String? _error;

  void _onAssetClassChanged(AssetClass value) {
    setState(() {
      _assetClass = value;
      _currency = value.defaultCurrency;
      _externalId = null;
    });
  }

  Future<void> _search() async {
    final ticker = _tickerController.text.trim().toUpperCase();
    if (ticker.isEmpty) return;

    setState(() {
      _searching = true;
      _error = null;
      _externalId = null;
    });

    try {
      final quote = await _dispatcher.fetchQuoteForTicker(ticker, _assetClass);
      setState(() {
        _nameController.text = quote.name ?? ticker;
        _currency = quote.currency ?? _assetClass.defaultCurrency;
        _externalId = quote.externalId;
      });
    } catch (_) {
      setState(() =>
          _error = 'Não foi possível buscar $ticker (${_assetClass.label}) — confira o ticker.');
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
      externalId: _externalId,
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
                    decoration: InputDecoration(
                      labelText: 'Ticker (${_assetClass.label})',
                      hintText: _assetClass.tickerHint,
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
                if (value != null) _onAssetClassChanged(value);
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

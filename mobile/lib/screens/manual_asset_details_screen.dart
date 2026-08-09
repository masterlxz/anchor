import 'package:flutter/material.dart';

import '../models/asset.dart';
import '../models/asset_valuation.dart';
import '../services/portfolio_repository.dart';

/// Mirror de `ManualAssetDetails.tsx` do desktop, sem anexos (exigem
/// file-picker + storage em disco, que o mobile não tem). `empresaNaoListada`
/// ganha o bloco de participação societária que `imovel` não tem — ambas
/// compartilham o histórico de avaliação.
class ManualAssetDetailsScreen extends StatefulWidget {
  final Asset asset;

  const ManualAssetDetailsScreen({super.key, required this.asset});

  @override
  State<ManualAssetDetailsScreen> createState() =>
      _ManualAssetDetailsScreenState();
}

class _ManualAssetDetailsScreenState extends State<ManualAssetDetailsScreen> {
  final _repository = PortfolioRepository();

  late Asset _asset;
  late TextEditingController _sharesOwnedController;
  late TextEditingController _totalSharesController;
  late TextEditingController _companyValuationController;

  final _valuationValueController = TextEditingController();
  final _valuationNotesController = TextEditingController();
  DateTime? _valuationDate;

  bool _loading = true;
  bool _savingEquity = false;
  bool _savingValuation = false;
  List<AssetValuation> _valuations = [];

  bool get _isEmpresaNaoListada =>
      _asset.assetClass == AssetClass.empresaNaoListada;

  @override
  void initState() {
    super.initState();
    _asset = widget.asset;
    _sharesOwnedController = TextEditingController(
      text: _asset.equitySharesOwned?.toString() ?? '',
    );
    _totalSharesController = TextEditingController(
      text: _asset.equityTotalShares?.toString() ?? '',
    );
    _companyValuationController = TextEditingController(
      text: _asset.equityCompanyValuation?.toString() ?? '',
    );
    for (final controller in [
      _sharesOwnedController,
      _totalSharesController,
      _companyValuationController,
    ]) {
      controller.addListener(() => setState(() {}));
    }
    _load();
  }

  Future<void> _load() async {
    final valuations = await _repository.listAssetValuations(_asset.id!);
    if (mounted) {
      setState(() {
        _valuations = valuations;
        _loading = false;
      });
    }
  }

  double? _parse(String text) => double.tryParse(text.replaceAll(',', '.'));

  Future<void> _saveEquity() async {
    setState(() => _savingEquity = true);

    final sharesOwned = _parse(_sharesOwnedController.text);
    final totalShares = _parse(_totalSharesController.text);
    final companyValuation = _parse(_companyValuationController.text);

    await _repository.updateAssetEquity(
      _asset.id!,
      sharesOwned: sharesOwned,
      totalShares: totalShares,
      companyValuation: companyValuation,
    );

    setState(() {
      _asset = Asset(
        id: _asset.id,
        ticker: _asset.ticker,
        name: _asset.name,
        assetClass: _asset.assetClass,
        currency: _asset.currency,
        createdAt: _asset.createdAt,
        externalId: _asset.externalId,
        equitySharesOwned: sharesOwned,
        equityTotalShares: totalShares,
        equityCompanyValuation: companyValuation,
      );
      _savingEquity = false;
    });
  }

  Future<void> _pickValuationDate() async {
    final picked = await showDatePicker(
      context: context,
      initialDate: _valuationDate ?? DateTime.now(),
      firstDate: DateTime(2000),
      lastDate: DateTime.now(),
    );
    if (picked != null) setState(() => _valuationDate = picked);
  }

  Future<void> _addValuation() async {
    final date = _valuationDate;
    final value = _parse(_valuationValueController.text);
    if (date == null || value == null || value <= 0) return;

    setState(() => _savingValuation = true);

    await _repository.insertAssetValuation(
      assetId: _asset.id!,
      valuationDate: date,
      value: value,
      notes: _valuationNotesController.text.trim().isEmpty
          ? null
          : _valuationNotesController.text.trim(),
    );

    _valuationDate = null;
    _valuationValueController.clear();
    _valuationNotesController.clear();
    setState(() => _savingValuation = false);
    await _load();
  }

  Future<void> _deleteValuation(int id) async {
    await _repository.deleteAssetValuation(id);
    await _load();
  }

  @override
  void dispose() {
    _sharesOwnedController.dispose();
    _totalSharesController.dispose();
    _companyValuationController.dispose();
    _valuationValueController.dispose();
    _valuationNotesController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final sharesOwned = _parse(_sharesOwnedController.text);
    final totalShares = _parse(_totalSharesController.text);
    final companyValuation = _parse(_companyValuationController.text);
    final percentual =
        (sharesOwned != null && totalShares != null && totalShares > 0)
        ? sharesOwned / totalShares
        : null;
    final participationValue = (percentual != null && companyValuation != null)
        ? percentual * companyValuation
        : null;

    final latestValue = _valuations.isEmpty ? null : _valuations.last.value;

    return Scaffold(
      appBar: AppBar(
        title: Text(
          _isEmpresaNaoListada ? 'Detalhes da empresa' : 'Detalhes do imóvel',
        ),
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : ListView(
              padding: const EdgeInsets.all(16),
              children: [
                if (_isEmpresaNaoListada) ...[
                  Text(
                    'Participação societária'
                    '${percentual != null ? ' (${(percentual * 100).toStringAsFixed(2)}%'
                              '${participationValue != null ? ' — ${participationValue.toStringAsFixed(2)} ${_asset.currency}' : ''})' : ''}',
                    style: Theme.of(context).textTheme.titleSmall,
                  ),
                  const SizedBox(height: 4),
                  Text(
                    'O percentual é sempre calculado a partir de cotas possuídas / total de '
                    'cotas da empresa — nunca digitado direto.',
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: _sharesOwnedController,
                    keyboardType: const TextInputType.numberWithOptions(
                      decimal: true,
                    ),
                    decoration: const InputDecoration(
                      labelText: 'Cotas/ações possuídas',
                    ),
                  ),
                  const SizedBox(height: 16),
                  TextField(
                    controller: _totalSharesController,
                    keyboardType: const TextInputType.numberWithOptions(
                      decimal: true,
                    ),
                    decoration: const InputDecoration(
                      labelText: 'Total de cotas/ações',
                    ),
                  ),
                  const SizedBox(height: 16),
                  TextField(
                    controller: _companyValuationController,
                    keyboardType: const TextInputType.numberWithOptions(
                      decimal: true,
                    ),
                    decoration: InputDecoration(
                      labelText: 'Valuation (${_asset.currency})',
                    ),
                  ),
                  const SizedBox(height: 12),
                  FilledButton(
                    onPressed: _savingEquity ? null : _saveEquity,
                    child: const Text('Salvar participação'),
                  ),
                  const Divider(height: 32),
                ],
                Text(
                  'Histórico de avaliação'
                  '${latestValue != null ? ' (última: ${latestValue.toStringAsFixed(2)} ${_asset.currency})' : ''}',
                  style: Theme.of(context).textTheme.titleSmall,
                ),
                const SizedBox(height: 12),
                if (_valuations.isEmpty)
                  const Text('Nenhuma avaliação registrada ainda.')
                else
                  for (final valuation in _valuations)
                    Card(
                      margin: const EdgeInsets.only(bottom: 8),
                      child: ListTile(
                        title: Text(
                          '${valuation.value.toStringAsFixed(2)} ${_asset.currency}',
                        ),
                        subtitle: Text(
                          '${valuation.valuationDate.day.toString().padLeft(2, '0')}/'
                          '${valuation.valuationDate.month.toString().padLeft(2, '0')}/'
                          '${valuation.valuationDate.year}'
                          '${valuation.notes != null ? ' · ${valuation.notes}' : ''}',
                        ),
                        trailing: IconButton(
                          icon: const Icon(Icons.delete_outline),
                          onPressed: () => _deleteValuation(valuation.id!),
                        ),
                      ),
                    ),
                const SizedBox(height: 16),
                ListTile(
                  contentPadding: EdgeInsets.zero,
                  title: Text(
                    _valuationDate != null
                        ? 'Data: ${_valuationDate!.day.toString().padLeft(2, '0')}/'
                              '${_valuationDate!.month.toString().padLeft(2, '0')}/${_valuationDate!.year}'
                        : 'Escolher data',
                  ),
                  trailing: const Icon(Icons.calendar_today),
                  onTap: _pickValuationDate,
                ),
                const SizedBox(height: 8),
                TextField(
                  controller: _valuationValueController,
                  keyboardType: const TextInputType.numberWithOptions(
                    decimal: true,
                  ),
                  decoration: InputDecoration(
                    labelText: 'Valor (${_asset.currency})',
                  ),
                ),
                const SizedBox(height: 16),
                TextField(
                  controller: _valuationNotesController,
                  decoration: const InputDecoration(
                    labelText: 'Notas (opcional)',
                  ),
                ),
                const SizedBox(height: 16),
                FilledButton(
                  onPressed: _savingValuation ? null : _addValuation,
                  child: const Text('Adicionar avaliação'),
                ),
              ],
            ),
    );
  }
}

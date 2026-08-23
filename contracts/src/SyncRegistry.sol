// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

// O SyncRegistry guarda apenas uma referência ao blob cifrado de sync (string
// opaca — hoje um ponteiro Arweave no formato "ar://<tx_id>", publicado via
// TruthID Sessão 87; o campo nunca mudou de forma, então nenhuma migration de
// contrato foi necessária quando o backend trocou de IPFS pra Arweave) — nunca
// o conteúdo em si. Diferente do VaultRegistry do TruthID,
// não existe indireção de identidade aqui: o registro é indexado direto pelo
// endereço que chama (a smart account do device), já que este contrato é de
// uso exclusivo do Practice Valuation, sem necessidade de um IdentityRegistry
// próprio nem de reaproveitar o do TruthID (rejeitado para esse fim).
contract SyncRegistry {
    // -------------------------------------------------------------------------
    // Tipos de dados
    // -------------------------------------------------------------------------

    struct CidRecord {
        string cid; // ponteiro opaco do blob cifrado atual (hoje "ar://<tx_id>" no Arweave)
        bytes32 contentHash; // keccak256 do blob (verificação de integridade)
        uint256 updatedAt; // block.timestamp da última atualização
        uint256 version; // contador monotônico — ordena atualizações
        bool exists;
    }

    // -------------------------------------------------------------------------
    // Estado
    // -------------------------------------------------------------------------

    // endereço (smart account) → registro atual
    mapping(address => CidRecord) private _records;

    // -------------------------------------------------------------------------
    // Eventos
    // -------------------------------------------------------------------------

    event RecordUpdated(address indexed who, string cid, bytes32 indexed contentHash, uint256 version);

    // -------------------------------------------------------------------------
    // Erros customizados
    // -------------------------------------------------------------------------

    error RecordNotFound(address who);
    error EmptyCid();
    error EmptyContentHash();

    // -------------------------------------------------------------------------
    // Funções de escrita
    // -------------------------------------------------------------------------

    /// Atualiza o registro de sync do chamador. Qualquer endereço pode
    /// atualizar só o próprio registro — não há conceito de "dono" além disso.
    function updateRecord(string calldata cid, bytes32 contentHash) external {
        if (bytes(cid).length == 0) revert EmptyCid();
        if (contentHash == bytes32(0)) revert EmptyContentHash();

        uint256 newVersion = _records[msg.sender].exists ? _records[msg.sender].version + 1 : 1;

        _records[msg.sender] = CidRecord({
            cid: cid,
            contentHash: contentHash,
            updatedAt: block.timestamp,
            version: newVersion,
            exists: true
        });

        emit RecordUpdated(msg.sender, cid, contentHash, newVersion);
    }

    // -------------------------------------------------------------------------
    // Funções de leitura
    // -------------------------------------------------------------------------

    /// Retorna o registro de sync atual de um endereço.
    function getRecord(address who) external view returns (CidRecord memory) {
        if (!_records[who].exists) revert RecordNotFound(who);
        return _records[who];
    }

    /// Retorna true se o endereço já tem um registro de sync.
    function hasRecord(address who) external view returns (bool) {
        return _records[who].exists;
    }
}

use {
    mollusk_svm::{
        program::{program_account, system_program},
        result::Check,
        Mollusk,
    },
    solana_sdk::{
        account::AccountSharedData,
        incinerator,
        instruction::{AccountMeta, Instruction, InstructionError},
        program_error::ProgramError,
        pubkey::Pubkey,
        system_instruction::SystemError,
        system_program,
    },
};

#[test]
fn test_write_data() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();

    let mollusk = Mollusk::new(&program_id, "test_program_primary");

    let data = &[1, 2, 3, 4, 5];
    let space = data.len();
    let lamports = mollusk.sysvars.rent.minimum_balance(space);

    let key = Pubkey::new_unique();
    let account = AccountSharedData::new(lamports, space, &program_id);

    let instruction = {
        let mut instruction_data = vec![1];
        instruction_data.extend_from_slice(data);
        Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![AccountMeta::new(key, true)],
        )
    };

    // Fail account not signer.
    {
        let mut account_not_signer_ix = instruction.clone();
        account_not_signer_ix.accounts[0].is_signer = false;

        mollusk.process_and_validate_instruction(
            &account_not_signer_ix,
            &[(key, account.clone())],
            &[
                Check::err(ProgramError::MissingRequiredSignature),
                Check::compute_units(279),
            ],
        );
    }

    // Fail data too large.
    {
        let mut data_too_large_ix = instruction.clone();
        data_too_large_ix.data = vec![1; space + 2];

        mollusk.process_and_validate_instruction(
            &data_too_large_ix,
            &[(key, account.clone())],
            &[
                Check::err(ProgramError::AccountDataTooSmall),
                Check::compute_units(290),
            ],
        );
    }

    // Success.
    mollusk.process_and_validate_instruction(
        &instruction,
        &[(key, account.clone())],
        &[
            Check::success(),
            Check::compute_units(358),
            Check::account(&key)
                .data(data)
                .lamports(lamports)
                .owner(&program_id)
                .build(),
        ],
    );
}

#[test]
fn test_transfer() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();

    let mollusk = Mollusk::new(&program_id, "test_program_primary");

    let payer = Pubkey::new_unique();
    let payer_lamports = 100_000_000;
    let payer_account = AccountSharedData::new(payer_lamports, 0, &system_program::id());

    let recipient = Pubkey::new_unique();
    let recipient_lamports = 0;
    let recipient_account = AccountSharedData::new(recipient_lamports, 0, &system_program::id());

    let transfer_amount = 2_000_000_u64;

    let instruction = {
        let mut instruction_data = vec![2];
        instruction_data.extend_from_slice(&transfer_amount.to_le_bytes());
        Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(payer, true),
                AccountMeta::new(recipient, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        )
    };

    // Fail payer not signer.
    {
        let mut payer_not_signer_ix = instruction.clone();
        payer_not_signer_ix.accounts[0].is_signer = false;

        mollusk.process_and_validate_instruction(
            &payer_not_signer_ix,
            &[
                (payer, payer_account.clone()),
                (recipient, recipient_account.clone()),
                system_program(),
            ],
            &[
                Check::err(ProgramError::MissingRequiredSignature),
                Check::compute_units(605),
            ],
        );
    }

    // Fail insufficient lamports.
    {
        mollusk.process_and_validate_instruction(
            &instruction,
            &[
                (payer, AccountSharedData::default()),
                (recipient, recipient_account.clone()),
                system_program(),
            ],
            &[
                Check::err(ProgramError::Custom(
                    SystemError::ResultWithNegativeLamports as u32,
                )),
                Check::compute_units(2261),
            ],
        );
    }

    // Success.
    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (payer, payer_account.clone()),
            (recipient, recipient_account.clone()),
            system_program(),
        ],
        &[
            Check::success(),
            Check::compute_units(2371),
            Check::account(&payer)
                .lamports(payer_lamports - transfer_amount)
                .build(),
            Check::account(&recipient)
                .lamports(recipient_lamports + transfer_amount)
                .build(),
        ],
    );
}

#[test]
fn test_close_account() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();

    let mollusk = Mollusk::new(&program_id, "test_program_primary");

    let key = Pubkey::new_unique();
    let account = AccountSharedData::new(50_000_000, 50, &program_id);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &[3],
        vec![
            AccountMeta::new(key, true),
            AccountMeta::new(incinerator::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    );

    // Fail account not signer.
    {
        let mut account_not_signer_ix = instruction.clone();
        account_not_signer_ix.accounts[0].is_signer = false;

        mollusk.process_and_validate_instruction(
            &account_not_signer_ix,
            &[
                (key, account.clone()),
                (incinerator::id(), AccountSharedData::default()),
                system_program(),
            ],
            &[
                Check::err(ProgramError::MissingRequiredSignature),
                Check::compute_units(605),
            ],
        );
    }

    // Success.
    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (key, account.clone()),
            (incinerator::id(), AccountSharedData::default()),
            system_program(),
        ],
        &[
            Check::success(),
            Check::compute_units(2563),
            Check::account(&key)
                .data(&[])
                .lamports(0)
                .owner(&system_program::id())
                .closed()
                .build(),
        ],
    );
}

#[test]
fn test_cpi() {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let program_id = Pubkey::new_unique();
    let cpi_target_program_id = Pubkey::new_unique();

    let mut mollusk = Mollusk::new(&program_id, "test_program_primary");

    let data = &[1, 2, 3, 4, 5];
    let space = data.len();
    let lamports = mollusk.sysvars.rent.minimum_balance(space);

    let key = Pubkey::new_unique();
    let account = AccountSharedData::new(lamports, space, &cpi_target_program_id);

    let instruction = {
        let mut instruction_data = vec![4];
        instruction_data.extend_from_slice(cpi_target_program_id.as_ref());
        instruction_data.extend_from_slice(data);
        Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(key, true),
                AccountMeta::new_readonly(cpi_target_program_id, false),
            ],
        )
    };

    // Fail CPI target program account not provided.
    {
        mollusk.process_and_validate_instruction(
            &instruction,
            &[(key, account.clone())],
            &[
                Check::err(ProgramError::NotEnoughAccountKeys),
                Check::compute_units(0),
            ],
        );
    }

    // Fail CPI target program not added to test environment.
    {
        mollusk.process_and_validate_instruction(
            &instruction,
            &[
                (key, account.clone()),
                (
                    cpi_target_program_id,
                    program_account(&cpi_target_program_id),
                ),
            ],
            &[
                // This is the error thrown by SVM. It also emits the message
                // "Program is not cached".
                Check::err(ProgramError::InvalidAccountData),
                Check::compute_units(1840),
            ],
        );
    }

    mollusk.add_program(&cpi_target_program_id, "test_program_cpi_target");

    // Fail account not signer.
    {
        let mut account_not_signer_ix = instruction.clone();
        account_not_signer_ix.accounts[0].is_signer = false;

        mollusk.process_and_validate_instruction(
            &account_not_signer_ix,
            &[
                (key, account.clone()),
                (
                    cpi_target_program_id,
                    program_account(&cpi_target_program_id),
                ),
            ],
            &[
                Check::instruction_err(InstructionError::PrivilegeEscalation), // CPI
                Check::compute_units(1841),
            ],
        );
    }

    // Fail data too large.
    {
        let mut data_too_large_ix = instruction.clone();
        let mut too_large_data = vec![4];
        too_large_data.extend_from_slice(cpi_target_program_id.as_ref());
        too_large_data.extend_from_slice(&vec![1; space + 2]);
        data_too_large_ix.data = too_large_data;

        mollusk.process_and_validate_instruction(
            &data_too_large_ix,
            &[
                (key, account.clone()),
                (
                    cpi_target_program_id,
                    program_account(&cpi_target_program_id),
                ),
            ],
            &[
                Check::err(ProgramError::AccountDataTooSmall),
                Check::compute_units(2162),
            ],
        );
    }

    // Success.
    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (key, account.clone()),
            (
                cpi_target_program_id,
                program_account(&cpi_target_program_id),
            ),
        ],
        &[
            Check::success(),
            Check::compute_units(2279),
            Check::account(&key)
                .data(data)
                .lamports(lamports)
                .owner(&cpi_target_program_id)
                .build(),
        ],
    );
}

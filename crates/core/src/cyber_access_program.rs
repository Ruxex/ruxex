use genex_api::AccessPrograms;
use genex_protocol::turn_input::CyberAccessProgram;

pub(crate) fn for_program(program: Option<CyberAccessProgram>) -> Option<AccessPrograms> {
    program.filter(|_| false).map(AccessPrograms::from)
}
